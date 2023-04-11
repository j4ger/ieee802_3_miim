#![no_std]
#![deny(missing_docs)]

//! A crate that provides access to the MIIM interface described
//! by IEEE standard 802.3

mod miim;

pub use miim::Miim;

#[cfg(feature = "mmd")]
mod mmd;
#[cfg(feature = "mmd")]
use mmd::Mmd;

#[cfg(feature = "ptp")]
mod ptp;
#[cfg(feature = "ptp")]
pub use ptp::PTP;

pub mod registers;
use registers::*;

#[cfg(feature = "phy")]
pub mod phy;

/// All basic link speeds possibly supported by the PHY.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkSpeed {
    /// 1000 Mbps
    Mpbs1000,
    /// 100 Mbps
    Mbps100,
    /// 10 Mbps
    Mpbs10,
    /// An illegal link speed is configured
    Illegal,
}

impl From<Bcr> for LinkSpeed {
    fn from(bcr: Bcr) -> Self {
        match (
            bcr.contains(Bcr::SPEED_SEL_MSB),
            bcr.contains(Bcr::SPEED_SEL_LSB),
        ) {
            (true, true) => LinkSpeed::Illegal,
            (true, false) => LinkSpeed::Mpbs1000,
            (false, true) => LinkSpeed::Mbps100,
            (false, false) => LinkSpeed::Mpbs10,
        }
    }
}

impl From<LinkSpeed> for Bcr {
    fn from(link_speed: LinkSpeed) -> Self {
        match link_speed {
            LinkSpeed::Mpbs1000 => Bcr::SPEED_SEL_MSB,
            LinkSpeed::Mbps100 => Bcr::SPEED_SEL_LSB,
            LinkSpeed::Mpbs10 => Bcr::empty(),
            LinkSpeed::Illegal => panic!("Cannot convert illegal link speed into Bcr"),
        }
    }
}

/// The status register of a PHY.
///
/// This struct describes what functions the PHY is capable of.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhyStatus {
    /// The PHY supports 100BASE-T4
    pub base100_t4: bool,
    /// The PHY supports 100BASE-X Full Duplex
    pub fd_100base_x: bool,
    /// The PHY supports 100BASE-X Half Duplex
    pub hd_100base_x: bool,
    /// The PHY supports 10 Mb/s full duplex
    pub fd_10mbps: bool,
    /// The PHY supports 10 Mb/s half duples
    pub hd_10mbps: bool,
    /// The PHY has extended status data in register 15
    pub extended_status: bool,
    /// The PHY supports unidirectional communication
    pub unidirectional: bool,
    /// The PHY is capable of accepting managmenet frames
    /// that are not preceded by the preamble
    pub preamble_suppression: bool,
    /// The PHY can perform autonegotiation
    pub autonegotiation: bool,
    /// The PHY supports extended capabilities, accessible
    /// through the extended register set
    pub extended_caps: bool,
}

impl PhyStatus {
    /// Create the best autonegotiation advertisement that we can.
    ///
    /// The returned advertisement will have default values for `selector_field` and `pause`.
    /// Those fields must be configured manually, or left to their defaults.
    pub fn best_autoneg_ad(&self) -> AutoNegotiationAdvertisement {
        let mut ad = AutoNegotiationAdvertisement::default();
        if self.base100_t4 {
            ad.base100_t4 = true;
        }

        if self.fd_100base_x {
            ad.fd_100base_tx = true;
        }

        if self.hd_100base_x {
            ad.hd_100base_tx = true;
        }

        if self.fd_10mbps {
            ad.fd_10base_t = true;
        }

        if self.hd_10mbps {
            ad.hd_10base_t = true;
        }

        ad
    }
}

impl From<Bsr> for PhyStatus {
    fn from(bsr: Bsr) -> Self {
        PhyStatus {
            base100_t4: bsr.contains(Bsr::_100BASET4),
            fd_100base_x: bsr.contains(Bsr::_100BASEXFD),
            hd_100base_x: bsr.contains(Bsr::_100BASEXHD),
            fd_10mbps: bsr.contains(Bsr::_10MPBSFD),
            hd_10mbps: bsr.contains(Bsr::_10MBPSHD),
            extended_status: bsr.contains(Bsr::EXTENDED_STATUS),
            unidirectional: bsr.contains(Bsr::UNIDRECTIONAL),
            preamble_suppression: bsr.contains(Bsr::MF_PREAMBLE_SUPPRESSION),
            autonegotiation: bsr.contains(Bsr::AUTONEG_ABLE),
            extended_caps: bsr.contains(Bsr::EXTENDED_CAPABILITIES),
        }
    }
}

/// The extended status register of a PHY.
///
/// This struct describes what extended functions the PHY is capable of.
///
/// This register is only valid if the field `extended_status` in the
///  [`PhyStatus`] describing this struct is `true`
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendedPhyStatus {
    /// The PHY supports 1000BASE-X Full Duplex
    pub fd_1000base_x: bool,
    /// The PHY supports 1000BASE-X Half Duplex
    pub hd_1000base_x: bool,
    /// The PHY supports 1000BASE-T Full Duplex
    pub fd_1000base_t: bool,
    /// The PHY supports 1000BASE-T Half Duplex
    pub hd_1000base_t: bool,
}

/// The selector field, describing the type of autonegotiation message
/// sent by a PHY.
///
/// In practice, [`SelectorField::Std802_3`] is used almost exclusively.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorField {
    /// The message is an IEEE Std 802.3 message
    Std802_3,
    /// The message is an IEEE Std 802.9 ISLAN-16T message
    Std802_9Islan16t,
    /// The message is an IEEE Std 802.5 message
    Std802_5,
    /// The message is an IEEE Std 1394 message
    Std1394,
}

impl Default for SelectorField {
    fn default() -> Self {
        Self::Std802_3
    }
}

impl From<AutoNegCap> for Option<SelectorField> {
    fn from(ana: AutoNegCap) -> Self {
        // We use bitwise XOR (`^`) here to ensure that all bits
        // we use to check for equivalence are set to their correct values

        let ana = ana & AutoNegCap::SEL_MASK;

        let field = if (ana ^ AutoNegCap::SEL_802_3).is_empty() {
            SelectorField::Std802_3
        } else if (ana ^ AutoNegCap::SEL_802_5).is_empty() {
            SelectorField::Std802_5
        } else if (ana ^ AutoNegCap::SEL_802_9_ISLAN_16T).is_empty() {
            SelectorField::Std802_9Islan16t
        } else if (ana ^ AutoNegCap::SEL_1394).is_empty() {
            SelectorField::Std1394
        } else {
            return None;
        };
        Some(field)
    }
}

impl From<SelectorField> for AutoNegCap {
    fn from(sf: SelectorField) -> Self {
        match sf {
            SelectorField::Std802_3 => AutoNegCap::SEL_802_3,
            SelectorField::Std802_9Islan16t => AutoNegCap::SEL_802_9_ISLAN_16T,
            SelectorField::Std802_5 => AutoNegCap::SEL_802_5,
            SelectorField::Std1394 => AutoNegCap::SEL_1394,
        }
    }
}

/// The PHY IDENT of this PHY
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhyIdent(u16, u16);

impl PhyIdent {
    /// Create a new PhyIdent
    pub fn new(phy_ident_1: u16, phy_ident_2: u16) -> Self {
        Self(phy_ident_1, phy_ident_2)
    }

    /// The raw values of this PhyIdent
    pub fn raw(&self) -> (u16, u16) {
        (self.0, self.1)
    }

    /// The raw value of this PhyIdent, as u32
    pub fn raw_u32(&self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32)
    }

    /// The OUI of this PhyIdent
    pub fn oui(&self) -> u32 {
        (self.0 as u32) << 6 & (self.1 as u32) >> 10
    }

    /// The model number of this PhyIdent
    pub fn model_number(&self) -> u8 {
        (self.1 >> 4) as u8 & 0x3F
    }

    /// The revision number of this PhyIdent
    pub fn revision(&self) -> u8 {
        (self.1) as u8 & 0x0F
    }
}

/// The pause mode supported by this PHY
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pause {
    /// The PHY supports no PAUSE modes
    NoPause,
    /// The PHY supports asymmetric PAUSE mode toward its link partner
    AsymmetricPartner,
    /// The PHY supports symmetric PAUSE mode
    Symmetric,
    /// The PHY supports both symmetric pause and asymmetric PAUSE towards
    /// the local device
    SymmetricAndAsymmetricLocal,
}

impl Default for Pause {
    fn default() -> Self {
        Pause::NoPause
    }
}

impl From<AutoNegCap> for Pause {
    fn from(ana: AutoNegCap) -> Self {
        match (
            ana.contains(AutoNegCap::ASSYMETRIC_PAUSE),
            ana.contains(AutoNegCap::PAUSE),
        ) {
            (false, false) => Pause::NoPause,
            (true, false) => Pause::AsymmetricPartner,
            (false, true) => Pause::Symmetric,
            (true, true) => Pause::SymmetricAndAsymmetricLocal,
        }
    }
}

impl From<Pause> for AutoNegCap {
    fn from(pause: Pause) -> Self {
        match pause {
            Pause::NoPause => AutoNegCap::empty(),
            Pause::AsymmetricPartner => AutoNegCap::ASSYMETRIC_PAUSE,
            Pause::Symmetric => AutoNegCap::PAUSE,
            Pause::SymmetricAndAsymmetricLocal => AutoNegCap::ASSYMETRIC_PAUSE | AutoNegCap::PAUSE,
        }
    }
}

/// An autonegotiation advertisement.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoNegotiationAdvertisement {
    /// The type of message sent
    pub selector_field: Option<SelectorField>,
    /// The PHY supports 10BASE-T
    pub hd_10base_t: bool,
    /// The PHY supports 10BASE-T Full Duplex
    pub fd_10base_t: bool,
    /// The PHY supports 100BASE-TX
    pub hd_100base_tx: bool,
    /// The PHY supports 100BASE-TX Full Duplex
    pub fd_100base_tx: bool,
    /// The PHY supports 100BASE-T4
    pub base100_t4: bool,
    /// The pause mode supported by the PHY
    pub pause: Pause,
}

impl Default for AutoNegotiationAdvertisement {
    fn default() -> Self {
        Self {
            selector_field: Some(SelectorField::default()),
            hd_10base_t: false,
            fd_10base_t: false,
            hd_100base_tx: false,
            fd_100base_tx: false,
            base100_t4: false,
            pause: Default::default(),
        }
    }
}

impl From<AutoNegCap> for AutoNegotiationAdvertisement {
    fn from(ana: AutoNegCap) -> Self {
        AutoNegotiationAdvertisement {
            selector_field: ana.into(),
            hd_10base_t: ana.contains(AutoNegCap::_10BASET),
            fd_10base_t: ana.contains(AutoNegCap::_10BASETFD),
            hd_100base_tx: ana.contains(AutoNegCap::_100BASETX),
            fd_100base_tx: ana.contains(AutoNegCap::_100BASETXFD),
            base100_t4: ana.contains(AutoNegCap::_100BASET4),
            pause: ana.into(),
        }
    }
}

/// An IEEE 802.3 compatible PHY
pub trait Phy<M: Miim> {
    /// The best advertisement this PHY can send out.
    ///
    /// "Best", in this case, means largest amount of supported features
    fn best_supported_advertisement(&self) -> AutoNegotiationAdvertisement;

    /// Get a mutable reference to the Media Independent Interface ([`Miim`]) for this PHY
    fn get_miim(&mut self) -> &mut M;

    /// Get the address of this PHY
    fn get_phy_addr(&self) -> u8;

    /// Read a PHY register over MIIM
    fn read(&mut self, address: u8) -> u16 {
        let phy = self.get_phy_addr();
        let miim = self.get_miim();
        miim.read(phy, address)
    }

    /// Write a PHY register over MIIM
    fn write(&mut self, address: u8, value: u16) {
        let phy = self.get_phy_addr();
        let miim = self.get_miim();
        miim.write(phy, address, value)
    }

    /// Get the raw value of the Base Control Register of this PHY
    fn bcr(&mut self) -> Bcr {
        Bcr::from_bits_truncate(self.read(Bcr::ADDRESS))
    }

    /// Modify the Base Control Register of this PHY
    fn modify_bcr<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Bcr),
    {
        let bcr = &mut self.bcr();
        f(bcr);
        self.write(Bcr::ADDRESS, bcr.bits());
    }

    /// Check if the PHY is currently resetting
    fn is_resetting(&mut self) -> bool {
        self.bcr().is_resetting()
    }

    /// Reset the PHY. Verify that the reset by checking
    /// [`Self::is_resetting`] == false before continuing usage
    fn reset(&mut self) {
        self.modify_bcr(|bcr| {
            bcr.reset(true);
        });
    }

    /// Perform a reset, blocking until the reset is completed
    fn blocking_reset(&mut self) {
        self.reset();
        while self.is_resetting() {}
    }

    /// Get the raw value of the Base Status Register of this PHY
    fn bsr(&mut self) -> Bsr {
        Bsr::from_bits_truncate(self.read(Bsr::ADDRESS))
    }

    /// Check if the PHY reports its link as being up
    fn phy_link_up(&mut self) -> bool {
        self.bsr().phy_link_up()
    }

    /// Check if the PHY reports its autonegotiation process
    /// as having completed
    fn autoneg_completed(&mut self) -> bool {
        self.bsr().autoneg_completed()
    }

    /// Read the status register for this PHY
    fn status(&mut self) -> PhyStatus {
        self.bsr().into()
    }

    /// Read the ESR for this PHY. Will return `None` if
    /// `extended_status` in [`Self::status`] is false.
    fn esr(&mut self) -> Option<Esr> {
        if self.status().extended_status {
            let phy = self.get_phy_addr();
            let miim = self.get_miim();
            Some(Esr::from_bits_truncate(miim.read(phy, Esr::ADDRESS)))
        } else {
            None
        }
    }

    /// Read the Extended Status Register for this PHY.
    ///
    /// Returns `None` if `extended_status` in [`Self::status`] is false.
    fn extended_status(&mut self) -> Option<ExtendedPhyStatus> {
        self.esr().map(|esr| ExtendedPhyStatus {
            fd_1000base_x: esr.contains(Esr::_1000BASEXFD),
            hd_1000base_x: esr.contains(Esr::_1000BASEXHD),
            fd_1000base_t: esr.contains(Esr::_1000BASETFD),
            hd_1000base_t: esr.contains(Esr::_1000BASETHD),
        })
    }

    /// Read the PHY identifier for this PHY.
    ///
    /// Returns `None` if `extended_capabilities` in [`Self::status`] is false
    fn phy_ident(&mut self) -> Option<PhyIdent> {
        if self.status().extended_caps {
            let msb = self.read(2);
            let lsb = self.read(3);
            Some(PhyIdent::new(msb, lsb))
        } else {
            None
        }
    }

    /// Set the autonegotiation advertisement and restarts the autonegotiation
    /// process
    ///
    /// This is a no-op if `extended_caps` in [`Self::status`] is false
    fn set_autonegotiation_advertisement(&mut self, ad: AutoNegotiationAdvertisement) {
        let status = self.status();
        if !status.extended_caps {
            return;
        }

        let mut ana = AutoNegCap::empty();

        if ad.hd_10base_t && status.hd_10mbps {
            ana.insert(AutoNegCap::_10BASET);
        }

        if ad.fd_10base_t && status.fd_10mbps {
            ana.insert(AutoNegCap::_10BASETFD);
        }

        if ad.hd_100base_tx && status.hd_100base_x {
            ana.insert(AutoNegCap::_100BASETX);
        }

        if ad.fd_100base_tx && status.fd_100base_x {
            ana.insert(AutoNegCap::_100BASETXFD);
        }

        if ad.base100_t4 {
            ana.insert(AutoNegCap::_100BASET4);
        }

        if let Some(selector) = ad.selector_field {
            ana.insert(selector.into());
        }

        ana.insert(ad.pause.into());

        self.write(AutoNegCap::LOCAL_CAP_ADDRESS, ana.bits());

        self.modify_bcr(|bcr| {
            bcr.set_autonegotiation(true).restart_autonegotiation();
        })
    }

    /// Get the advertised capabilities of this PHY
    ///
    /// This is a no-op if `extended_caps` in [`Self::status`] is false
    fn get_autonegotiation_caps(&mut self) -> Option<AutoNegotiationAdvertisement> {
        let status = self.status();
        if !status.extended_caps {
            return None;
        }
        let ana = AutoNegCap::from_bits_truncate(self.read(AutoNegCap::LOCAL_CAP_ADDRESS));
        Some(ana.into())
    }

    /// Get the capabilites of the autonegotiation partner of this PHY
    ///
    /// This is a no-op if `extended_caps` in [`Self::status`] is false
    fn get_autonegotiation_partner_caps(&mut self) -> Option<AutoNegotiationAdvertisement> {
        let status = self.status();
        if !status.extended_caps {
            return None;
        }
        let ana = AutoNegCap::from_bits_truncate(self.read(AutoNegCap::PARTNER_CAP_ADDRESS));
        Some(ana.into())
    }

    /// This returns `None` if `extended_caps` in `Self::status` is `false`
    fn ane(&mut self) -> Option<Ane> {
        if self.status().extended_caps {
            Some(Ane::from_bits_truncate(self.read(Ane::ADDRESS)))
        } else {
            None
        }
    }

    /// Read an MMD register
    #[cfg(feature = "mmd")]
    fn mmd_read(&mut self, mmd_address: u8, reg_address: u16) -> u16
    where
        Self: Sized,
    {
        Mmd::read(self, mmd_address, reg_address)
    }

    /// Write an MMD register
    #[cfg(feature = "mmd")]
    fn mmd_write(&mut self, device_address: u8, reg_address: u16, reg_value: u16)
    where
        Self: Sized,
    {
        Mmd::write(self, device_address, reg_address, reg_value)
    }
}
