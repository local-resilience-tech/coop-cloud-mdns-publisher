use std::time::Duration;
use tokio::time::timeout;
use zbus::{connection, proxy};

const AVAHI_TIMEOUT: Duration = Duration::from_secs(3);

#[proxy(
    interface = "org.freedesktop.Avahi.Server",
    default_service = "org.freedesktop.Avahi",
    default_path = "/"
)]
trait AvahiServer {
    async fn get_state(&self) -> zbus::Result<i32>;
    async fn get_version_string(&self) -> zbus::Result<String>;
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
    proxy: AvahiServerProxy<'static>,
}

impl AvahiClient {
    pub async fn connect() -> Result<Self, AvahiError> {
        let builder = connection::Builder::system()
            .map_err(|e| AvahiError::DBusConnect(format!("could not create D-Bus connection builder: {e}")))?;

        let conn = timeout(AVAHI_TIMEOUT, builder.method_timeout(AVAHI_TIMEOUT).build())
            .await
            .map_err(|_| AvahiError::DBusConnect("timed out connecting to system D-Bus".to_string()))?
            .map_err(|e| AvahiError::DBusConnect(format!("could not connect to system D-Bus: {e}")))?;

        let proxy = AvahiServerProxy::new(&conn)
            .await
            .map_err(|e| AvahiError::ProxyCreate(format!("could not reach Avahi on D-Bus: {e}")))?;

        Ok(AvahiClient { proxy })
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

        // AVAHI_IF_UNSPEC = -1, AVAHI_PROTO_UNSPEC = -1, AVAHI_PROTO_INET = 0
        let local_address = self
            .proxy
            .resolve_host_name(-1, -1, &fqdn, 0, 0)
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
}
