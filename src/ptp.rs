/// Trait for hardware support of Presicion Time Protocol (IEEE1588)
pub trait PTP {
    /// Enable PTP Clock
    fn start_ptp(&mut self);
    /// Disable PTP Clock
    fn stop_ptp(&mut self);

    /// Set PTP Clock
    fn set_clock(&mut self, clock: u16);
    /// Read PTP Clock
    fn read_clock(&mut self) -> u16;
    /// Reset PTP Clock
    fn reset_clock(&mut self);

    /// Check if PTP Clock is started
    fn started(&mut self) -> bool;

    /// Set rate control value
    fn set_rate_control(&mut self, rate: u32);
}
