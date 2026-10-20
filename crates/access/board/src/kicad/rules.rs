//! KiCad's custom design rules (`.kicad_dru`), as its documentation and its
//! DRC describe them: `(version 1)`, then `(rule NAME (layer …) (condition
//! "…") (constraint KIND (min …) (opt …) (max …)) (severity …))`.

use super::project::matches_wildcards;
use super::sexpr;
use crate::BoardError;
use crate::sexp::Sexp;
use traxide_contracts::{
    Condition, Constraint, CustomRule, Item, Layer, LayerId, Limits, Net, NetClass, NetClassId,
    NetId,
};

/// What the rules' names refer to on the board.
pub(crate) struct Names<'a> {
    pub classes: &'a [NetClass],
    pub nets: &'a [Net],
    pub layers: &'a [Layer],
}

/// The rules in `text`, in their order, one per constraint. Constraints traxide
/// does not route by are left out, and so are rules KiCad does not check.
pub(crate) fn read(text: &str, names: &Names) -> Result<Vec<CustomRule>, BoardError> {
    let uncommented: String = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|line| {
            if line.trim_start().starts_with('#') {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    // The file is a sequence of lists; one around them keeps the line numbers.
    let root = sexpr::parse(&format!("(kicad_dru {uncommented}\n)"))?;
    let version = root.child("version").and_then(|v| v.atoms().next());
    if version != Some("1") {
        return Err(root.invalid("not KiCad custom rules: expected (version 1)"));
    }
    let mut rules = Vec::new();
    for rule in root.children("rule") {
        let ignored = rule
            .child("severity")
            .and_then(|s| s.atoms().next())
            .is_some_and(|s| s == "ignore");
        if ignored {
            continue;
        }
        let name = rule.arg_atom(0, "rule name")?;
        let mut condition = match rule.child("condition").and_then(|c| c.atoms().next()) {
            Some(expression) => condition(expression, names),
            None => Condition::Always,
        };
        if let Some(layer) = rule.child("layer").and_then(|l| l.atoms().next()) {
            condition = Condition::All(vec![on_layer(layer, names.layers), condition]);
        }
        for constraint in rule.children("constraint") {
            let Some(kind) = constraint.atoms().next().and_then(constraint_kind) else {
                continue;
            };
            rules.push(CustomRule {
                name: name.to_owned(),
                condition: condition.clone(),
                constraint: kind,
                limits: Limits {
                    min: limit(constraint, "min")?,
                    opt: limit(constraint, "opt")?,
                    max: limit(constraint, "max")?,
                },
            });
        }
    }
    Ok(rules)
}

fn constraint_kind(keyword: &str) -> Option<Constraint> {
    Some(match keyword {
        "clearance" => Constraint::Clearance,
        "track_width" => Constraint::TrackWidth,
        "via_diameter" => Constraint::ViaDiameter,
        "hole_size" => Constraint::HoleSize,
        "edge_clearance" => Constraint::EdgeClearance,
        "hole_clearance" => Constraint::HoleClearance,
        _ => return None,
    })
}

fn limit(constraint: &Sexp, keyword: &str) -> Result<Option<i32>, BoardError> {
    let Some(limit) = constraint.child(keyword) else {
        return Ok(None);
    };
    let text = limit.arg_atom(0, keyword)?;
    length(text).map(Some).ok_or_else(|| {
        limit.invalid(format!(
            "cannot read the length {text:?}: traxide reads a number in mm, mil or in"
        ))
    })
}

/// A length such as `0.2mm`, `"1 mm"` or `8mil`, in nanometres. KiCad's DRC
/// ignores the whole file over a length without a unit, so it is an error here too.
fn length(text: &str) -> Option<i32> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+')))
        .unwrap_or(text.len());
    let number: f64 = text[..split].parse().ok()?;
    let nanometres_per_unit = match text[split..].trim() {
        "mm" => 1e6,
        "mil" => 25_400.0,
        "in" => 25.4e6,
        _ => return None,
    };
    Some((number * nanometres_per_unit).round() as i32)
}

/// The rule's layer: `outer`, `inner` or a layer's name. A name that is not a
/// copper layer's might be another name for one, so the rule may apply.
fn on_layer(layer: &str, layers: &[Layer]) -> Condition {
    let last = layers.len().saturating_sub(1);
    let indices: Vec<usize> = match layer {
        "outer" if last == 0 => vec![0],
        "outer" => vec![0, last],
        "inner" => (1..last).collect(),
        name => match layers.iter().position(|l| l.name == name) {
            Some(index) => vec![index],
            None => return Condition::Unknown,
        },
    };
    Condition::OnLayers(indices.into_iter().map(LayerId::from_index).collect())
}

/// The condition `expression` means: comparisons of `NetClass` and `NetName`,
/// joined with `&&`, `||` and `!`. Any other test may hold or not.
fn condition(expression: &str, names: &Names) -> Condition {
    let Some(tokens) = tokens(expression) else {
        return Condition::Unknown;
    };
    let mut parser = Parser {
        tokens: &tokens,
        at: 0,
        names,
    };
    match parser.any() {
        Some(condition) if parser.at == tokens.len() => condition,
        _ => Condition::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Text(String),
    Symbol(&'static str),
}

const SYMBOLS: [&str; 16] = [
    "==", "!=", "&&", "||", "<=", ">=", "!", "(", ")", ",", "<", ">", "+", "-", "*", "/",
];

/// `None` for text that is no expression.
fn tokens(expression: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut rest = expression;
    while let Some(c) = rest.chars().next() {
        if c.is_whitespace() {
            rest = &rest[c.len_utf8()..];
        } else if c == '\'' || c == '"' {
            let end = rest[1..].find(c)? + 1;
            tokens.push(Token::Text(rest[1..end].to_owned()));
            rest = &rest[end + 1..];
        } else if let Some(symbol) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) {
            tokens.push(Token::Symbol(symbol));
            rest = &rest[symbol.len()..];
        } else if c.is_alphanumeric() || c == '_' || c == '.' {
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
                .unwrap_or(rest.len());
            tokens.push(Token::Word(rest[..end].to_owned()));
            rest = &rest[end..];
        } else {
            return None;
        }
    }
    Some(tokens)
}

struct Parser<'t> {
    tokens: &'t [Token],
    at: usize,
    names: &'t Names<'t>,
}

/// One side of a comparison.
enum Operand {
    Property(String),
    Text(String),
    Other,
}

impl Parser<'_> {
    fn eat(&mut self, symbol: &str) -> bool {
        let found = matches!(self.tokens.get(self.at), Some(Token::Symbol(s)) if *s == symbol);
        self.at += usize::from(found);
        found
    }

    fn any(&mut self) -> Option<Condition> {
        let mut any = vec![self.all()?];
        while self.eat("||") {
            any.push(self.all()?);
        }
        Some(if any.len() == 1 {
            any.remove(0)
        } else {
            Condition::Any(any)
        })
    }

    fn all(&mut self) -> Option<Condition> {
        let mut all = vec![self.unary()?];
        while self.eat("&&") {
            all.push(self.unary()?);
        }
        Some(if all.len() == 1 {
            all.remove(0)
        } else {
            Condition::All(all)
        })
    }

    fn unary(&mut self) -> Option<Condition> {
        if self.eat("!") {
            return Some(Condition::Not(Box::new(self.unary()?)));
        }
        if self.eat("(") {
            let inner = self.any()?;
            return self.eat(")").then_some(inner);
        }
        let left = self.operand()?;
        let comparison = ["==", "!=", "<=", ">=", "<", ">"]
            .into_iter()
            .find(|op| self.eat(op));
        let Some(op) = comparison else {
            return Some(Condition::Unknown);
        };
        let right = self.operand()?;
        let equal = match (left, right) {
            (Operand::Property(property), Operand::Text(text))
            | (Operand::Text(text), Operand::Property(property)) => self.equals(&property, &text),
            _ => Condition::Unknown,
        };
        Some(match op {
            "==" => equal,
            "!=" => Condition::Not(Box::new(equal)),
            _ => Condition::Unknown,
        })
    }

    fn operand(&mut self) -> Option<Operand> {
        let token = self.tokens.get(self.at)?.clone();
        self.at += 1;
        match token {
            Token::Word(_) if self.eat("(") => {
                // A function such as A.inDiffPair('*'): its arguments are skipped.
                let mut depth = 1;
                while depth > 0 {
                    match self.tokens.get(self.at)? {
                        Token::Symbol("(") => depth += 1,
                        Token::Symbol(")") => depth -= 1,
                        _ => {}
                    }
                    self.at += 1;
                }
                Some(Operand::Other)
            }
            Token::Word(word) => Some(Operand::Property(word)),
            Token::Text(text) => Some(Operand::Text(text)),
            Token::Symbol(_) => None,
        }
    }

    /// `property == pattern`, where KiCad matches the pattern's wildcards
    /// without regard to case.
    fn equals(&self, property: &str, pattern: &str) -> Condition {
        let pattern = pattern.to_lowercase();
        let matches = |name: &str| matches_wildcards(&pattern, &name.to_lowercase());
        let (item, property) = match property.split_once('.') {
            Some(("A", property)) => (Item::A, property),
            Some(("B", property)) => (Item::B, property),
            _ => return Condition::Unknown,
        };
        match property {
            "NetClass" => Condition::NetClassIn {
                item,
                classes: (self.names.classes.iter().enumerate())
                    .filter(|(_, class)| matches(&class.name))
                    .map(|(i, _)| NetClassId::from_index(i))
                    .collect(),
            },
            "NetName" => Condition::NetIn {
                item,
                nets: (matches("").then_some(None).into_iter())
                    .chain(
                        (self.names.nets.iter().enumerate())
                            .filter(|(_, net)| matches(&net.name))
                            .map(|(i, _)| Some(NetId::from_index(i))),
                    )
                    .collect(),
            },
            _ => Condition::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str) -> NetClass {
        NetClass {
            name: name.into(),
            track_width: 0,
            clearance: 0,
            via_kinds: vec![],
        }
    }

    fn net(name: &str) -> Net {
        Net {
            name: name.into(),
            class: NetClassId(0),
            pads: vec![],
        }
    }

    fn layer(name: &str) -> Layer {
        Layer {
            name: name.into(),
            routable: true,
        }
    }

    fn rules(text: &str) -> Vec<CustomRule> {
        let classes = [class("Default"), class("HV"), class("USB")];
        let nets = [net("GND"), net("/HV+"), net("/HV-")];
        let layers = ["F.Cu", "In1.Cu", "In2.Cu", "B.Cu"].map(layer);
        let names = Names {
            classes: &classes,
            nets: &nets,
            layers: &layers,
        };
        read(&format!("(version 1)\n{text}"), &names).expect("valid rules")
    }

    fn only(text: &str) -> CustomRule {
        let mut rules = rules(text);
        assert_eq!(rules.len(), 1, "{rules:?}");
        rules.remove(0)
    }

    fn when(condition: &str) -> Condition {
        only(&format!(
            "(rule r (constraint clearance (min 1mm)) (condition \"{condition}\"))"
        ))
        .condition
    }

    fn classes(item: Item, classes: &[u32]) -> Condition {
        Condition::NetClassIn {
            item,
            classes: classes.iter().copied().map(NetClassId).collect(),
        }
    }

    #[test]
    fn a_rule_states_its_constraint_and_limits_in_nanometres() {
        let rule = only(
            "(rule \"HV clearance\" (constraint clearance (min 1.5mm) (opt \"2 mm\") (max 0.1in)))",
        );
        assert_eq!(rule.name, "HV clearance");
        assert_eq!(rule.constraint, Constraint::Clearance);
        assert_eq!(rule.condition, Condition::Always);
        assert_eq!(
            rule.limits,
            Limits {
                min: Some(1_500_000),
                opt: Some(2_000_000),
                max: Some(2_540_000),
            }
        );
        assert_eq!(
            only("(rule r (constraint track_width (min 8mil)))")
                .limits
                .min,
            Some(203_200)
        );
    }

    #[test]
    fn a_rule_with_several_constraints_becomes_one_rule_each_in_order() {
        let read = rules(
            "(rule r (constraint track_width (min 0.3mm)) (constraint disallow via) \
             (constraint edge_clearance (min 1mm)) (constraint hole_size (max 0.4mm)))",
        );
        let kinds: Vec<Constraint> = read.iter().map(|r| r.constraint).collect();
        assert_eq!(
            kinds,
            [
                Constraint::TrackWidth,
                Constraint::EdgeClearance,
                Constraint::HoleSize
            ]
        );
    }

    #[test]
    fn rules_kicad_does_not_check_are_left_out() {
        assert!(rules("(rule r (severity ignore) (constraint clearance (min 1mm)))").is_empty());
    }

    #[test]
    fn comments_are_skipped() {
        assert_eq!(
            rules("# wider\n  # still a comment\n(rule r (constraint clearance (min 1mm)))").len(),
            1
        );
    }

    #[test]
    fn net_classes_match_by_wildcard_without_regard_to_case() {
        assert_eq!(when("A.NetClass == 'HV'"), classes(Item::A, &[1]));
        assert_eq!(when("B.NetClass == 'hv'"), classes(Item::B, &[1]));
        assert_eq!(when("'?SB' == A.NetClass"), classes(Item::A, &[2]));
        assert_eq!(when("A.NetClass == 'Power'"), classes(Item::A, &[]));
    }

    #[test]
    fn net_names_match_by_wildcard_and_copper_of_no_net_has_an_empty_name() {
        let nets = |nets: &[Option<u32>]| Condition::NetIn {
            item: Item::A,
            nets: nets.iter().map(|n| n.map(NetId)).collect(),
        };
        assert_eq!(when("A.NetName == '/HV*'"), nets(&[Some(1), Some(2)]));
        assert_eq!(when("A.NetName == 'gnd'"), nets(&[Some(0)]));
        assert_eq!(when("A.NetName == ''"), nets(&[None]));
    }

    #[test]
    fn conditions_combine_with_and_or_not_and_parentheses() {
        assert_eq!(
            when("A.NetClass == 'HV' && B.NetClass != 'HV' || !(A.NetClass == 'USB')"),
            Condition::Any(vec![
                Condition::All(vec![
                    classes(Item::A, &[1]),
                    Condition::Not(Box::new(classes(Item::B, &[1]))),
                ]),
                Condition::Not(Box::new(classes(Item::A, &[2]))),
            ])
        );
    }

    #[test]
    fn tests_traxide_cannot_tell_may_hold_or_not() {
        assert_eq!(when("A.Type == 'Via'"), Condition::Unknown);
        assert_eq!(
            when("A.inDiffPair('*') && A.NetClass == 'HV'"),
            Condition::All(vec![Condition::Unknown, classes(Item::A, &[1])])
        );
        assert_eq!(when("A.NetClass == 'HV' &&"), Condition::Unknown);
        assert_eq!(when("A.NetClass == 'HV' ; 1"), Condition::Unknown);
    }

    #[test]
    fn a_layer_limits_a_rule_to_copper_on_it() {
        let on = |layer: &str| {
            only(&format!(
                "(rule r (layer {layer}) (constraint clearance (min 1mm)))"
            ))
            .condition
        };
        let layers = |ids: &[u32]| {
            Condition::All(vec![
                Condition::OnLayers(ids.iter().copied().map(LayerId).collect()),
                Condition::Always,
            ])
        };
        assert_eq!(on("outer"), layers(&[0, 3]));
        assert_eq!(on("inner"), layers(&[1, 2]));
        assert_eq!(on("\"In2.Cu\""), layers(&[2]));
        assert_eq!(
            on("\"Top\""),
            Condition::All(vec![Condition::Unknown, Condition::Always])
        );
    }

    #[test]
    fn rules_without_a_version_or_with_lengths_without_units_are_refused() {
        let names = Names {
            classes: &[],
            nets: &[],
            layers: &[],
        };
        assert!(read("(rule r (constraint clearance (min 1mm)))", &names).is_err());
        assert!(matches!(
            read(
                "(version 1)\n(rule r\n (constraint clearance (min 1)))",
                &names
            ),
            Err(BoardError::Invalid { line: 3, .. })
        ));
        assert!(
            read(
                "(version 1) (rule r (constraint clearance (min 1um)))",
                &names
            )
            .is_err()
        );
    }
}
