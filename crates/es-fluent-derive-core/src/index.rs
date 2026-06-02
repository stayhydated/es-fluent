//! Typed indexes used while translating Rust fields into Fluent arguments.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclarationIndex(usize);

impl DeclarationIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TupleFieldIndex(usize);

impl TupleFieldIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExposedArgumentIndex(usize);

impl ExposedArgumentIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

pub trait FieldArgumentIndex {
    fn argument_index(self) -> usize;
}

impl FieldArgumentIndex for DeclarationIndex {
    fn argument_index(self) -> usize {
        self.as_usize()
    }
}

impl FieldArgumentIndex for TupleFieldIndex {
    fn argument_index(self) -> usize {
        self.as_usize()
    }
}

impl FieldArgumentIndex for ExposedArgumentIndex {
    fn argument_index(self) -> usize {
        self.as_usize()
    }
}
