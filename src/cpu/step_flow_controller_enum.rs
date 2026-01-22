pub enum StepFlowController {
    Continue,
    EarlyReturn(u8), // cycles consumed
}
