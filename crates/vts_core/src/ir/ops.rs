#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    Not,
}

#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, Debug)]
pub enum ConstOp {
    Unit,
    Zero,
}

#[derive(Clone, Copy, Debug)]
pub enum AnyOp {
    Unary(UnaryOp),
    Binary(BinaryOp),
    Const(ConstOp),
    Mux,
}

impl AnyOp {
    pub fn not() -> Self {
        Self::from(UnaryOp::Not)
    }

    pub fn and() -> Self {
        Self::from(BinaryOp::And)
    }

    pub fn or() -> Self {
        Self::from(BinaryOp::Or)
    }

    pub fn xor() -> Self {
        Self::from(BinaryOp::Xor)
    }

    pub fn unit() -> Self {
        Self::from(ConstOp::Unit)
    }

    pub fn zero() -> Self {
        Self::from(ConstOp::Zero)
    }

    pub fn mux() -> Self {
        Self::Mux
    }
}

impl From<UnaryOp> for AnyOp {
    fn from(op: UnaryOp) -> Self {
        Self::Unary(op)
    }
}

impl From<BinaryOp> for AnyOp {
    fn from(op: BinaryOp) -> Self {
        Self::Binary(op)
    }
}

impl From<ConstOp> for AnyOp {
    fn from(op: ConstOp) -> Self {
        Self::Const(op)
    }
}
