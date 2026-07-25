use std::time::Duration;
use tokio::time::timeout;
use zbus::{connection, proxy};

const AVAHI_TIMEOUT: Duration = Duration::from_secs(3);
const INTERFACE: Interface = Interface::Unspec;
const PROTOCOL: Protocol = Protocol::Inet;

bitflags::bitflags! {
    /// Flags for Avahi publish operations (mirrors `AvahiPublishFlags` in avahi-common/defs.h).
    #[derive(Clone, Copy, Default)]
    pub struct PublishFlags: u32 {
        const UNIQUE         = 0x0001;
        const NO_PROBE       = 0x0002;
        const NO_ANNOUNCE    = 0x0004;
        const ALLOW_MULTIPLE = 0x0008;
        const NO_REVERSE     = 0x0010;
        const NO_COOKIE      = 0x0020;
        const UPDATE         = 0x0040;
        const USE_WIDE_AREA  = 0x0080;
        const USE_MULTICAST  = 0x0100;
    }
}

/// Network interface index. Use `Interface::Unspec` to mean "all interfaces".
#[derive(Clone, Copy)]
pub enum Interface {
    Unspec,
    Index(i32),
}

impl From<Interface> for i32 {
    fn from(i: Interface) -> i32 {
        match i {
            Interface::Unspec => -1,
            Interface::Index(n) => n,
        }
    }
}

/// Address family / protocol (mirrors `AvahiProtocol` in avahi-common/address.h).
#[derive(Clone, Copy)]
pub enum Protocol {
    /// Any address family.
    Unspec,
    /// IPv4 only.
    Inet,
    /// IPv6 only.
    Inet6,
}

impl From<Protocol> for i32 {
    fn from(p: Protocol) -> i32 {
        match p {
            Protocol::Unspec => -1,
            Protocol::Inet => 0,
            Protocol::Inet6 => 1,
        }
    }
}

#[proxy(
    interface = "org.freedesktop.Avahi.Server",
    default_service = "org.freedesktop.Avahi",
    default_path = "/"
)]
trait AvahiServer {
    async fn get_state(&self) -> zbus::Result<i32>;
    async fn get_version_string(&self) -> zbus::Result<String>;
    async fn get_host_name(&self) -> zbus::Result<String>;
    async fn get_host_name_fqdn(&self) -> zbus::Result<String>;
    #[allow(clippy::too_many_arguments)]
    async fn resolve_host_name(
        &self,
        interface: i32,
        protocol: i32,
        name: &str,
        aprotocol: i32,
        flags: u32,
    ) -> zbus::Result<(i32, i32, String, i32, String, u32)>;
    async fn entry_group_new(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

/// An opaque handle to a published mDNS entry group. Drop to withdraw the records.
pub struct PublishedRecord(#[allow(dead_code)] AvahiEntryGroupProxy<'static>);

#[proxy(
    interface = "org.freedesktop.Avahi.EntryGroup",
    default_service = "org.freedesktop.Avahi"
)]
trait AvahiEntryGroup {
    async fn add_address(
        &self,
        interface: i32,
        protocol: i32,
        flags: u32,
        name: &str,
        address: &str,
    ) -> zbus::Result<()>;
    async fn commit(&self) -> zbus::Result<()>;
}

#[derive(Debug)]
pub enum AvahiError {
    DBusConnect(String),
    ProxyCreate(String),
    Call(String),
}

impl std::fmt::Display for AvahiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvahiError::DBusConnect(msg) => write!(f, "{msg}"),
            AvahiError::ProxyCreate(msg) => write!(f, "{msg}"),
            AvahiError::Call(msg) => write!(f, "{msg}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvahiState {
    Invalid,
    Registering,
    Running,
    Collision,
    Failure,
    Unknown(i32),
}

impl AvahiState {
    fn from_raw(raw: i32) -> Self {
        match raw {
            0 => AvahiState::Invalid,
            1 => AvahiState::Registering,
            2 => AvahiState::Running,
            3 => AvahiState::Collision,
            4 => AvahiState::Failure,
            n => AvahiState::Unknown(n),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            AvahiState::Invalid => "Invalid",
            AvahiState::Registering => "Registering",
            AvahiState::Running => "Running",
            AvahiState::Collision => "Collision",
            AvahiState::Failure => "Failure",
            AvahiState::Unknown(_) => "Unknown",
        }
    }
}

pub struct AvahiStatus {
    pub version: String,
    pub state: AvahiState,
    pub fqdn: String,
    pub local_address: Option<String>,
}

pub struct AvahiClient {
    conn: zbus::Connection,
    proxy: AvahiServerProxy<'static>,
}

impl AvahiClient {
    pub async fn connect() -> Result<Self, AvahiError> {
        let builder = connection::Builder::system().map_err(|e| {
            AvahiError::DBusConnect(format!("could not create D-Bus connection builder: {e}"))
        })?;

        let conn = timeout(AVAHI_TIMEOUT, builder.method_timeout(AVAHI_TIMEOUT).build())
            .await
            .map_err(|_| {
                AvahiError::DBusConnect("timed out connecting to system D-Bus".to_string())
            })?
            .map_err(|e| {
                AvahiError::DBusConnect(format!("could not connect to system D-Bus: {e}"))
            })?;

        let proxy = AvahiServerProxy::new(&conn)
            .await
            .map_err(|e| AvahiError::ProxyCreate(format!("could not reach Avahi on D-Bus: {e}")))?;

        Ok(AvahiClient { conn, proxy })
    }

    pub async fn status(&self) -> Result<AvahiStatus, AvahiError> {
        let version = self
            .proxy
            .get_version_string()
            .await
            .map_err(|e| AvahiError::Call(format!("Avahi did not respond: {e}")))?;

        let state = self
            .proxy
            .get_state()
            .await
            .map_err(|e| AvahiError::Call(format!("could not get Avahi state: {e}")))?;

        let fqdn = self
            .proxy
            .get_host_name_fqdn()
            .await
            .unwrap_or_else(|_| "(unknown)".to_string());

        // NO_REVERSE suppresses the reverse PTR record, avoiding collisions when
        // multiple names resolve to the same IP.
        let local_address = self
            .proxy
            .resolve_host_name(
                INTERFACE.into(),
                Protocol::Unspec.into(),
                &fqdn,
                PROTOCOL.into(),
                PublishFlags::empty().bits(),
            )
            .await
            .ok()
            .map(|(_, _, _, _, addr, _)| addr);

        Ok(AvahiStatus {
            version,
            state: AvahiState::from_raw(state),
            fqdn,
            local_address,
        })
    }

    pub async fn get_host_name(&self) -> Result<String, AvahiError> {
        self.proxy
            .get_host_name()
            .await
            .map_err(|e| AvahiError::Call(format!("could not get hostname: {e}")))
    }

    /// Publish an A record mapping `name` (e.g. `myapp-thirdfish.local`) to `address`.
    /// Returns an `AvahiEntryGroupProxy` that must be kept alive for the record to remain published.
    pub async fn publish_address(
        &self,
        name: &str,
        address: &str,
    ) -> Result<PublishedRecord, AvahiError> {
        let path = self
            .proxy
            .entry_group_new()
            .await
            .map_err(|e| AvahiError::Call(format!("could not create entry group: {e}")))?;

        let group = AvahiEntryGroupProxy::builder(&self.conn)
            .path(path)
            .map_err(|e| AvahiError::Call(format!("invalid entry group path: {e}")))?
            .build()
            .await
            .map_err(|e| AvahiError::Call(format!("could not create entry group proxy: {e}")))?;

        // NO_REVERSE suppresses the reverse PTR record, avoiding collisions when
        // multiple names resolve to the same IP.
        group
            .add_address(
                INTERFACE.into(),
                PROTOCOL.into(),
                PublishFlags::NO_REVERSE.bits(),
                name,
                address,
            )
            .await
            .map_err(|e| AvahiError::Call(format!("could not add address record: {e}")))?;

        group
            .commit()
            .await
            .map_err(|e| AvahiError::Call(format!("could not commit entry group: {e}")))?;

        Ok(PublishedRecord(group))
    }
}
