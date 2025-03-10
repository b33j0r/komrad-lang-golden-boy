pub mod uuid7 {
    use std::fmt::Display;
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Uuid7(pub Uuid);

    impl Uuid7 {
        pub fn new() -> Self {
            Self(Uuid::now_v7())
        }
    }

    impl Display for Uuid7 {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

pub mod literal {
    pub type Int = i64;
    pub type UInt = u64;
    pub type Float = f64;
    pub type Bytes = Vec<u8>;
}
