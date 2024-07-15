pub struct Netlist {
    pub(super) model_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum LatchTrigger {
    /// Alias "re".
    RisingEdge,
    /// Alias "fe".
    FallingEdge,
    /// Alias "ah".
    ActiveHigh,
    /// Alias "al".
    ActiveLow,
    /// Alias "as".
    Async,
}

impl std::str::FromStr for LatchTrigger {
    type Err = String;

    /// Try to interpret `input` as a latch trigger.
    ///
    /// Returns
    /// - `Ok(LatchTrigger)` if input is any of the latch trigger aliases
    /// - `Err(input.to_string())` otherwise
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "re" => Ok(Self::RisingEdge),
            "fe" => Ok(Self::FallingEdge),
            "ah" => Ok(Self::ActiveHigh),
            "al" => Ok(Self::ActiveLow),
            "as" => Ok(Self::Async),
            _ => Err(input.to_string()),
        }
    }
}
