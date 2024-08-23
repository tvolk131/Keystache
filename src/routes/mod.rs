use std::{collections::VecDeque, str::FromStr, sync::Arc};

use add_nostr_keypair::AddNostrKeypair;
use home::Home;
use iced::{
    widget::{column, row, text, Column, Text},
    Element,
};
use nip_55::nip_46::Nip46RequestApproval;
use nostr_sdk::{
    secp256k1::{Keypair, Secp256k1},
    SecretKey,
};
use unlock::Unlock;

use crate::{
    db::Database,
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, Message,
};

mod add_nostr_keypair;
mod home;
mod unlock;

#[derive(Clone)]
pub enum Route {
    Unlock(Unlock),
    Home(Home),
    AddNostrKeypair(AddNostrKeypair),
}

impl<'a> Route {
    pub fn new_locked() -> Self {
        Self::Unlock(Unlock {
            password: String::new(),
            is_secure: true,
            db_already_exists: Database::exists(),
        })
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::UnlockPasswordInputChanged(new_password) => {
                if let Self::Unlock(Unlock { password, .. }) = self {
                    *password = new_password;
                }
            }
            Message::UnlockToggleSecureInput => {
                if let Self::Unlock(Unlock { is_secure, .. }) = self {
                    *is_secure = !*is_secure;
                }
            }
            Message::UnlockPasswordSubmitted => {
                if let Self::Unlock(Unlock { password, .. }) = self {
                    if let Ok(db) = Database::open_or_create_in_app_data_dir(password) {
                        let db = Arc::new(db);

                        *self = Self::Home(Home {
                            connected_state: ConnectedState {
                                db,
                                in_flight_nip46_requests: VecDeque::new(),
                            },
                        });
                    }
                }
            }
            Message::DbDeleteAllData => {
                if let Self::Unlock(Unlock {
                    db_already_exists, ..
                }) = self
                {
                    Database::delete();
                    *db_already_exists = false;
                }
            }
            Message::GoToHomePage => {
                if let Some(connected_state) = self.get_connected_state() {
                    *self = Self::Home(Home {
                        connected_state: connected_state.clone(),
                    });
                }
            }
            Message::GoToAddKeypairPage => {
                if let Self::Home(Home { connected_state }) = self {
                    *self = Self::AddNostrKeypair(AddNostrKeypair {
                        connected_state: connected_state.clone(),
                        nsec: String::new(),
                        keypair_or: None,
                    });
                }
            }
            Message::SaveKeypair => {
                if let Self::AddNostrKeypair(AddNostrKeypair {
                    connected_state,
                    keypair_or: Some(keypair),
                    ..
                }) = self
                {
                    // TODO: Surface this error to the UI.
                    let _ = connected_state.db.save_keypair(keypair);
                }
            }
            Message::SaveKeypairNsecInputChanged(new_nsec) => {
                if let Self::AddNostrKeypair(AddNostrKeypair {
                    nsec, keypair_or, ..
                }) = self
                {
                    *nsec = new_nsec;

                    // Set `keypair_or` to `Some` if `nsec` is a valid secret key, `None` otherwise.
                    *keypair_or = SecretKey::from_str(nsec).map_or(None, |secret_key| {
                        Some(Keypair::from_secret_key(&Secp256k1::new(), &secret_key))
                    });
                }
            }
            Message::IncomingNip46Request(data) => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    connected_state.in_flight_nip46_requests.push_back(data);
                }
            }
            Message::ApproveFirstIncomingNip46Request => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Approve).unwrap();
                    }
                }
            }
            Message::RejectFirstIncomingNip46Request => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Reject).unwrap();
                    }
                }
            }
        };
    }

    pub fn view(&self) -> Element<Message> {
        // If there are any incoming NIP46 requests, display the first one over the rest of the UI.
        if let Some(connected_state) = self.get_connected_state() {
            if let Some(req) = connected_state.in_flight_nip46_requests.front() {
                return Column::new()
                    .push(Text::new("Incoming NIP-46 request"))
                    .push(Text::new(format!("{:?}", req.0)))
                    .push(row![
                        icon_button("Approve", SvgIcon::ThumbUp, PaletteColor::Primary,)
                            .on_press(Message::ApproveFirstIncomingNip46Request),
                        icon_button("Reject", SvgIcon::ThumbDown, PaletteColor::Primary,)
                            .on_press(Message::RejectFirstIncomingNip46Request),
                    ])
                    .into();
            }
        }

        match self {
            Self::Unlock(unlock) => unlock.view(),
            Self::Home(home) => home.view(),
            Self::AddNostrKeypair(add_nostr_keypair) => add_nostr_keypair.view(),
        }
        .into()
    }

    pub fn get_connected_state(&self) -> Option<&ConnectedState> {
        match self {
            Self::Unlock { .. } => None,
            Self::Home(Home { connected_state }) => Some(connected_state),
            Self::AddNostrKeypair(AddNostrKeypair {
                connected_state, ..
            }) => Some(connected_state),
        }
    }

    fn get_connected_state_mut(&mut self) -> Option<&mut ConnectedState> {
        match self {
            Self::Unlock { .. } => None,
            Self::Home(Home { connected_state }) => Some(connected_state),
            Self::AddNostrKeypair(AddNostrKeypair {
                connected_state, ..
            }) => Some(connected_state),
        }
    }
}

pub fn container<'a>(title: &str) -> Column<'a, Message> {
    column![text(title.to_string()).size(35)]
        .spacing(20)
        .align_items(iced::Alignment::Center)
}
