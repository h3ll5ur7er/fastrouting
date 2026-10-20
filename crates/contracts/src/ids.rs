macro_rules! dense_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn from_index(index: usize) -> Self {
                Self(u32::try_from(index).expect("fewer than 2³² items"))
            }
        }
    };
}

dense_id!(
    /// A copper layer, by its position in the stack from the top.
    LayerId
);
dense_id!(NetId);
dense_id!(NetClassId);
dense_id!(PadId);
dense_id!(ViaKindId);
