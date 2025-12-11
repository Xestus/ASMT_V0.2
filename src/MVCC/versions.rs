#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct Version {
    pub value: String,
    pub xmin: u32,
    pub xmax: Option<u32>,
}