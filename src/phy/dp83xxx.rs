//! Phy implementation for the TI DP83xxx Series

use crate::{ptp::PTP, registers::Esr, AutoNegotiationAdvertisement, ExtendedPhyStatus, Miim, Phy};

use self::registers::{PHYSTS, PTPCTL};

use super::{AdvancedPhySpeed, PhySpeed, PhyWithSpeed};

/// A DP83xxx series PHY
#[derive(Debug)]
pub struct DP83XXX<MIIM: Miim, const PTP: bool> {
    phy_addr: u8,
    miim: MIIM,
}

/// DP83640 with hardware PTP stamping
pub type DP83640<MIIM> = DP83XXX<MIIM, true>;

/// DP83848
pub type DP83848<MIIM> = DP83XXX<MIIM, false>;

impl<MIIM: Miim, const PTP_EN: bool> DP83XXX<MIIM, PTP_EN> {
    const PAGE_REG: u8 = 0x13;

    const INTERRUPT_REG: (u16, u8) = (0x00, 0x1B);
    const INTERRUPT_REG_EN_LINK_CHANGE: u16 = 1 << 5;
    /// A mask for determining if the Link Status Change Interrupt occurred
    pub const INTERRUPT_REG_INT_LINK_CHANGE: u16 = 1 << 13;

    /// Create a new Ksz8081r at `phy_addr`, backed by the given `miim`,
    pub fn new(miim: MIIM, phy_addr: u8) -> Self {
        Self { phy_addr, miim }
    }

    /// Enable the link status change interrupt
    pub fn interrupt_enable(&mut self) {
        self.write_ext(Self::INTERRUPT_REG, Self::INTERRUPT_REG_EN_LINK_CHANGE);
    }

    /// Get the link speed at which the PHY is currently operating
    pub fn link_speed(&mut self) -> Option<PhySpeed> {
        let phy_ctrl1 = PHYSTS::from_bits_truncate(self.read(PHYSTS::ADDRESS));
        phy_ctrl1.into()
    }

    /// Get the value of the interrupt register.
    pub fn get_interrupt_reg_val(&mut self) -> u16 {
        self.read_ext(Self::INTERRUPT_REG)
    }

    /// Check whether a link is established or not
    pub fn link_established(&mut self) -> bool {
        self.autoneg_completed() && self.phy_link_up()
    }

    /// Release the underlying [`Miim`]
    pub fn release(self) -> MIIM {
        self.miim
    }

    pub fn write_ext(&mut self, address_ext: (u16, u8), value: u16) {
        self.write(Self::PAGE_REG, address_ext.0);
        self.write(address_ext.1, value);
    }

    pub fn read_ext(&mut self, address_ext: (u16, u8)) -> u16 {
        self.write(Self::PAGE_REG, address_ext.0);
        self.read(address_ext.1)
    }
}

impl<MIIM: Miim, const PTP_EN: bool> Phy<MIIM> for DP83XXX<MIIM, PTP_EN> {
    fn best_supported_advertisement(&self) -> AutoNegotiationAdvertisement {
        AutoNegotiationAdvertisement {
            hd_10base_t: true,
            fd_10base_t: true,
            hd_100base_tx: true,
            fd_100base_tx: true,
            base100_t4: true,
            ..Default::default()
        }
    }

    fn get_miim(&mut self) -> &mut MIIM {
        &mut self.miim
    }

    fn get_phy_addr(&self) -> u8 {
        self.phy_addr
    }

    fn esr(&mut self) -> Option<Esr> {
        None
    }

    fn extended_status(&mut self) -> Option<ExtendedPhyStatus> {
        None
    }
}

impl<MIIM: Miim, const PTP_EN: bool> PhyWithSpeed<MIIM> for DP83XXX<MIIM, PTP_EN> {
    fn get_link_speed(&mut self) -> Option<AdvancedPhySpeed> {
        self.link_speed().map(Into::into)
    }
}

#[allow(missing_docs)]
pub mod registers {
    use bitflags::bitflags;

    use crate::phy::PhySpeed;

    bitflags! {
        // PHYSTS contains device status
        pub struct PHYSTS: u16 {
            const FULL_DUPLEX = (1<<2);
            const MBIT10=(1<<1);
            const LINK_STATUS=(1<<0);
        }
    }

    impl PHYSTS {
        pub const ADDRESS: u8 = 0x19;
    }

    impl From<PHYSTS> for Option<PhySpeed> {
        fn from(ctrl: PHYSTS) -> Self {
            let full_duplex = ctrl.contains(PHYSTS::FULL_DUPLEX);
            let mbit_10 = ctrl.contains(PHYSTS::MBIT10);
            let link = ctrl.contains(PHYSTS::LINK_STATUS);

            if !link {
                return None;
            }

            let speed = match (full_duplex, mbit_10) {
                (true, true) => PhySpeed::FullDuplexBase10T,
                (true, false) => PhySpeed::FullDuplexBase100Tx,
                (false, true) => PhySpeed::HalfDuplexBase10T,
                (false, false) => PhySpeed::HalfDuplexBase100Tx,
            };
            Some(speed)
        }
    }

    bitflags! {
        pub struct PTPCTL:u16{
            const PTP_RESET = (1<<0);
            const PTP_DISABLE = (1<<1);
            const PTP_ENABLE = (1<<2);
            const PTP_LOAD_CLK = (1<<4);
            const PTP_RD_CLK = (1<<5);
        }
    }

    impl PTPCTL {
        pub const ADDRESS: (u16, u8) = (0b100, 0x14);
    }
}

impl<MIIM: Miim> DP83640<MIIM> {
    const PTP_TIME: (u16, u8) = (0b100, 0x15);
    const PTP_RATEL: (u16, u8) = (0b100, 0x18);
    const PTP_RATEH: (u16, u8) = (0b100, 0x19);
}

impl<MIIM: Miim> PTP for DP83640<MIIM> {
    fn started(&mut self) -> bool {
        let ptpctl = PTPCTL::from_bits_truncate(self.read_ext(PTPCTL::ADDRESS));
        ptpctl.contains(PTPCTL::PTP_ENABLE)
    }

    fn reset_clock(&mut self) {
        let mut ptpctl = PTPCTL::from_bits_truncate(self.read_ext(PTPCTL::ADDRESS));
        ptpctl.set(PTPCTL::PTP_RESET, true);
        self.write_ext(PTPCTL::ADDRESS, ptpctl.bits());
    }

    fn start_ptp(&mut self) {
        let mut ptpctl = PTPCTL::from_bits_truncate(self.read_ext(PTPCTL::ADDRESS));
        ptpctl.set(PTPCTL::PTP_ENABLE, true);
        self.write_ext(PTPCTL::ADDRESS, ptpctl.bits());
    }

    fn stop_ptp(&mut self) {
        let mut ptpctl = PTPCTL::from_bits_truncate(self.read_ext(PTPCTL::ADDRESS));
        ptpctl.set(PTPCTL::PTP_DISABLE, true);
        self.write_ext(PTPCTL::ADDRESS, ptpctl.bits());
    }

    fn set_clock(&mut self, clock: u16) {
        let mut ptpctl = PTPCTL::from_bits_truncate(self.read_ext(PTPCTL::ADDRESS));
        ptpctl.set(PTPCTL::PTP_LOAD_CLK, true);

        self.write_ext(Self::PTP_TIME, clock);

        self.write_ext(PTPCTL::ADDRESS, ptpctl.bits());
    }

    fn read_clock(&mut self) -> u16 {
        let mut ptpctl = PTPCTL::from_bits_truncate(self.read_ext(PTPCTL::ADDRESS));
        ptpctl.set(PTPCTL::PTP_RD_CLK, true);
        self.write_ext(PTPCTL::ADDRESS, ptpctl.bits());

        self.read_ext(Self::PTP_TIME)
    }

    fn set_rate_control(&mut self, rate: u32) {
        let high_bits = (rate >> 16) as u16;
        let low_bits = rate as u16;

        self.write_ext(Self::PTP_RATEH, high_bits);
        self.write_ext(Self::PTP_RATEL, low_bits);
    }
}
