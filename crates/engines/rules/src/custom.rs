//! The board's custom rules: which apply to which copper, and what they ask.

use std::collections::{BTreeMap, BTreeSet};
use traxide_contracts::{Condition, Constraint, Design, Item, Limits, NetClassId, NetId};

/// Whether a condition holds, as far as can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Truth {
    No,
    Maybe,
    Yes,
}

impl Truth {
    fn of(holds: bool) -> Self {
        if holds { Self::Yes } else { Self::No }
    }

    fn not(self) -> Self {
        match self {
            Self::No => Self::Yes,
            Self::Maybe => Self::Maybe,
            Self::Yes => Self::No,
        }
    }
}

/// Copper the rules treat alike: of one class, and in the same of the sets
/// of nets that rules name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Group {
    pub class: NetClassId,
    /// Any one net of the group; `None` is copper of no net.
    member: Option<NetId>,
}

/// The groups of a design: they replace its classes in the compiled tables, so
/// that a rule naming a net reaches that net alone.
pub(crate) struct Groups {
    pub groups: Vec<Group>,
    pub of_net: Vec<NetClassId>,
    pub of_no_net: NetClassId,
}

impl Groups {
    pub fn of(design: &Design) -> Self {
        let mut named = Vec::new();
        for rule in &design.rules.custom {
            named_nets(&rule.condition, &mut named);
        }
        let mut groups = Vec::new();
        let mut index = BTreeMap::new();
        let mut group = |class, member: Option<NetId>| {
            let sets: Vec<bool> = named.iter().map(|set| set.contains(&member)).collect();
            *index.entry((class, sets)).or_insert_with(|| {
                groups.push(Group { class, member });
                NetClassId::from_index(groups.len() - 1)
            })
        };
        let of_net = design
            .nets
            .iter()
            .enumerate()
            .map(|(i, net)| group(net.class, Some(NetId::from_index(i))))
            .collect();
        let of_no_net = group(design.rules.default_class, None);
        Self {
            groups,
            of_net,
            of_no_net,
        }
    }
}

fn named_nets(condition: &Condition, named: &mut Vec<BTreeSet<Option<NetId>>>) {
    match condition {
        Condition::NetIn { nets, .. } => named.push(nets.iter().copied().collect()),
        Condition::Not(inner) => named_nets(inner, named),
        Condition::All(all) | Condition::Any(all) => {
            for inner in all {
                named_nets(inner, named);
            }
        }
        Condition::Always
        | Condition::OnLayers(_)
        | Condition::NetClassIn { .. }
        | Condition::Unknown => {}
    }
}

/// The rules of one constraint that concern some copper: the last that surely
/// applies, and those after it that may.
pub(crate) struct Applying<'r> {
    pub last: Option<&'r Limits>,
    pub possible: Vec<&'r Limits>,
}

impl Applying<'_> {
    /// The minimum asked: the last rule's, else `fallback`, and no less than
    /// any rule that may apply asks.
    pub fn minimum(&self, fallback: i32) -> i32 {
        let stated = self.last.and_then(|l| l.min).unwrap_or(fallback);
        self.possible
            .iter()
            .filter_map(|l| l.min)
            .fold(stated, i32::max)
    }

    /// The value to use: the last rule's optimum, else `fallback`, kept within
    /// the limits of every rule that may apply.
    pub fn value(&self, fallback: i32) -> i32 {
        let stated = self
            .last
            .map_or(fallback, |l| within(l.opt.unwrap_or(fallback), l));
        self.possible
            .iter()
            .fold(stated, |value, limits| within(value, limits))
    }

    /// Whether `value` (`None`: unknown) keeps the limits of every rule that may apply.
    pub fn allows(&self, value: Option<i32>) -> bool {
        self.last
            .into_iter()
            .chain(self.possible.iter().copied())
            .all(|limits| match value {
                Some(value) => within(value, limits) == value,
                None => limits.min.is_none() && limits.max.is_none(),
            })
    }
}

fn within(value: i32, limits: &Limits) -> i32 {
    let raised = limits.min.map_or(value, |min| value.max(min));
    limits.max.map_or(raised, |max| raised.min(max))
}

pub(crate) struct Rules<'d> {
    design: &'d Design,
    groups: &'d [Group],
}

impl<'d> Rules<'d> {
    pub fn new(design: &'d Design, groups: &'d [Group]) -> Self {
        Self { design, groups }
    }

    /// The rules of `constraint` between copper of groups `a` and `b`. KiCad
    /// tries a condition both ways round. A clearance rule without a minimum
    /// asks nothing of the distance.
    pub fn between(&self, constraint: Constraint, a: NetClassId, b: NetClassId) -> Applying<'d> {
        let (a, b) = (&self.groups[a.index()], &self.groups[b.index()]);
        self.applying(
            constraint,
            |limits| limits.min.is_some(),
            |condition| {
                self.holds(condition, a, Some(b))
                    .max(self.holds(condition, b, Some(a)))
            },
        )
    }

    /// The rules of `constraint` for copper of group `a` alone.
    pub fn of(&self, constraint: Constraint, a: NetClassId) -> Applying<'d> {
        let a = &self.groups[a.index()];
        self.applying(
            constraint,
            |_| true,
            |condition| self.holds(condition, a, None),
        )
    }

    fn applying(
        &self,
        constraint: Constraint,
        counts: impl Fn(&Limits) -> bool,
        holds: impl Fn(&Condition) -> Truth,
    ) -> Applying<'d> {
        let mut possible = Vec::new();
        let rules = self.design.rules.custom.iter().rev();
        for rule in rules.filter(|r| r.constraint == constraint && counts(&r.limits)) {
            match holds(&rule.condition) {
                Truth::Yes => {
                    return Applying {
                        last: Some(&rule.limits),
                        possible,
                    };
                }
                Truth::Maybe => possible.push(&rule.limits),
                Truth::No => {}
            }
        }
        Applying {
            last: None,
            possible,
        }
    }

    /// Whether `condition` holds for copper of `a` against copper of `b`;
    /// `b` is `None` where a constraint concerns `a` alone.
    fn holds(&self, condition: &Condition, a: &Group, b: Option<&Group>) -> Truth {
        let item = |item| match item {
            Item::A => Some(a),
            Item::B => b,
        };
        match condition {
            Condition::Always => Truth::Yes,
            Condition::OnLayers(layers) if layers.is_empty() => Truth::No,
            Condition::OnLayers(layers) if layers.len() >= self.design.layers.len() => Truth::Yes,
            Condition::OnLayers(_) | Condition::Unknown => Truth::Maybe,
            Condition::NetClassIn { item: i, classes } => {
                item(*i).map_or(Truth::Maybe, |g| Truth::of(classes.contains(&g.class)))
            }
            Condition::NetIn { item: i, nets } => {
                item(*i).map_or(Truth::Maybe, |g| Truth::of(nets.contains(&g.member)))
            }
            Condition::Not(inner) => self.holds(inner, a, b).not(),
            Condition::All(all) => all
                .iter()
                .map(|c| self.holds(c, a, b))
                .min()
                .unwrap_or(Truth::Yes),
            Condition::Any(any) => any
                .iter()
                .map(|c| self.holds(c, a, b))
                .max()
                .unwrap_or(Truth::No),
        }
    }
}
