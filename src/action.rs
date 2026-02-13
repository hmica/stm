use uuid::Uuid;

/// All possible actions in the application (TEA pattern).
pub enum Action {
    Tick,
    Render,
    Quit,
    NavigateUp,
    NavigateDown,
    Select,
    SwitchPanel,
    StartSearch,
    SearchInput(char),
    SearchBackspace,
    EndSearch,
    ShowHelp,

    // Connection actions
    Connect(usize),
    ConnectionEstablished,
    ConnectionFailed(String),
    Disconnect,
    Disconnected,

    // Tunnel actions
    ShowAddTunnelModal,
    ModalInput(char),
    ModalBackspace,
    ModalNextField,
    ModalSubmit,
    TunnelFailed(String),
    ToggleTunnel(usize),
    TunnelToggled(Uuid, bool),
    DeleteTunnel(usize),
    TunnelDeleted(Uuid),

    // Persistence
    RestoreTunnels,
}
