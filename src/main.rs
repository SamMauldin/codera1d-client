#![feature(drain_filter)]
#![windows_subsystem = "windows"]

use anyhow::Result;
use chrono::{serde::ts_seconds, DateTime, Utc};
use iced::{
    time,
    widget::{
        button::Button, container::Container, pick_list, text_input::TextInput, Column, Row, Text,
    },
    Alignment, Application, Command, Element, Length, Settings, Subscription,
};
use iced_style::Theme;
use if_chain::if_chain;
use inputbot::KeybdKey;
use lazy_static::lazy_static;
use reqwest::{header, Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod keybind;

lazy_static! {
    static ref KEYBOARD_MUTEX: Mutex<()> = Mutex::new(());
}

const ENDPOINT: &'static str = env!("CODERA1D_ENDPOINT");
const API_KEY: &'static str = env!("CODERA1D_API_KEY");

pub fn main() -> iced::Result {
    println!("hi");
    let mut settings = Settings::default();
    settings.window.size = (400, 150);
    settings.window.always_on_top = true;
    settings.window.resizable = false;
    CodeRaid::run(settings)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReservation {
    pub codes: Vec<String>,
    #[serde(with = "ts_seconds")]
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullRaid {
    pub remaining_codes: Vec<String>,
    pub tried_codes: Vec<String>,
    pub code_reservations: Vec<CodeReservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialRaid {
    pub remaining_code_count: u32,
    pub tried_code_count: u32,
}

struct CodeRaid {
    connected: bool,
    client: Arc<Client>,
    current_raid: Option<String>,
    raids: HashMap<String, PartialRaid>,
    code_reservations: HashMap<String, Vec<CodeReservation>>,
    last_codes: Vec<String>,
    create_raid_name: String,
    codes_tried: i32,
}

#[derive(Debug, Clone)]
enum Message {
    RaidsUpdated(Option<HashMap<String, PartialRaid>>),
    CodesReserved(Option<(String, CodeReservation)>),
    SelectRaid(Option<String>),
    CreateRaidInputChanged(String),
    CreateRaid,
    DeleteRaid,
    Refresh,
    HotkeyPressed,
    Dummy,
}

impl Application for CodeRaid {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (CodeRaid, Command<Message>) {
        let mut headers = header::HeaderMap::new();
        headers.insert("X-Api-Key", header::HeaderValue::from_str(API_KEY).unwrap());
        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        let client_arc = Arc::new(client);
        let client_ref = client_arc.clone();

        (
            CodeRaid {
                connected: Default::default(),
                client: client_arc,
                current_raid: Default::default(),
                raids: Default::default(),
                code_reservations: Default::default(),
                last_codes: Default::default(),
                create_raid_name: Default::default(),
                codes_tried: Default::default(),
            },
            Command::perform(CodeRaid::fetch_raids(client_ref), Message::RaidsUpdated),
        )
    }

    fn theme(&self) -> Self::Theme {
        iced::Theme::Dark
    }

    fn title(&self) -> String {
        format!(
            "codera1d v{} - {}",
            env!("CARGO_PKG_VERSION"),
            self.current_raid
                .clone()
                .unwrap_or_else(|| "No raid selected".to_owned())
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.cleanup_state();

        let command = match message {
            Message::RaidsUpdated(res) => {
                self.connected = res.is_some();

                if let Some(raids) = res {
                    self.raids = raids;
                }

                Command::none()
            }
            Message::SelectRaid(raid) => {
                self.current_raid = raid;

                Command::none()
            }
            Message::CodesReserved(res) => {
                if let Some((raid_name, reservation)) = res {
                    let entry = self
                        .code_reservations
                        .entry(raid_name)
                        .or_insert_with(|| Vec::new());

                    entry.push(reservation);
                }

                Command::none()
            }
            Message::Refresh => {
                let mut commands = Vec::new();

                commands.push(Command::perform(
                    CodeRaid::fetch_raids(self.client.clone()),
                    Message::RaidsUpdated,
                ));

                if let Some(raid) = &self.current_raid {
                    let code_reservations = self
                        .code_reservations
                        .entry(raid.to_owned())
                        .or_insert_with(|| Vec::new());

                    let codes_reserved = code_reservations
                        .iter()
                        .map(|res| res.codes.len())
                        .sum::<usize>();

                    if codes_reserved < 5 {
                        commands.push(Command::perform(
                            CodeRaid::reserve_codes(raid.to_owned(), self.client.clone()),
                            Message::CodesReserved,
                        ));
                    }
                }

                Command::batch(commands)
            }
            Message::HotkeyPressed => {
                let mutex = KEYBOARD_MUTEX.try_lock().ok();
                if_chain! {
                    if let Some(_) = mutex;

                    if let Some(raid) = &self.current_raid;
                    let code_reservations = self
                            .code_reservations
                            .entry(raid.to_owned())
                            .or_insert_with(|| Vec::new());

                    if let Some(reservation) = code_reservations.first_mut();
                    if let Some(code) = reservation.codes.pop();

                    then {
                        input_code(&code);

                        self.last_codes.insert(0, code.clone());
                        self.last_codes.truncate(5);

                        self.codes_tried += 1;

                        return Command::perform(
                            CodeRaid::try_code(raid.to_owned(), code, self.client.clone()),
                            |_| Message::Dummy,
                        );
                    }
                }

                Command::none()
            }
            Message::CreateRaidInputChanged(raid) => {
                self.create_raid_name = raid.clone();
                self.create_raid_name = self.create_raid_name.to_lowercase();
                self.create_raid_name = self.create_raid_name.replace(" ", "_");

                self.create_raid_name
                    .retain(|char| r#"abcdefghijklmnopqrstuvwxyz1234567890_"#.contains(char));

                self.create_raid_name.truncate(20);

                Command::none()
            }
            Message::CreateRaid => {
                if !self.create_raid_name.is_empty() {
                    let cmd = Command::perform(
                        CodeRaid::create_raid(self.create_raid_name.clone(), self.client.clone()),
                        |_| Message::Dummy,
                    );

                    self.create_raid_name = String::new();

                    cmd
                } else {
                    Command::none()
                }
            }
            Message::DeleteRaid => {
                if let Some(current_raid_name) = &self.current_raid {
                    Command::perform(
                        CodeRaid::delete_raid(current_raid_name.clone(), self.client.clone()),
                        |_| Message::Dummy,
                    )
                } else {
                    Command::none()
                }
            }
            Message::Dummy => Command::none(),
        };

        self.cleanup_state();

        command
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(1)).map(|_| Message::Refresh),
            Subscription::from_recipe(keybind::Keybind {}).map(|_| Message::HotkeyPressed),
        ])
    }

    fn view(&self) -> Element<Message> {
        let mut raid_options = self
            .raids
            .keys()
            .map(|key| key.to_owned())
            .collect::<Vec<_>>();

        raid_options.push(String::new());
        raid_options.sort();

        let mut pick_row = Row::new().spacing(20).push(
            pick_list(raid_options, self.current_raid.clone(), |selected_raid| {
                if selected_raid == "" {
                    Message::SelectRaid(None)
                } else {
                    Message::SelectRaid(Some(selected_raid))
                }
            })
            .width(Length::Fixed(320.0)),
        );

        if self.current_raid.is_some() {
            pick_row = pick_row.push(
                Button::new(
                    Text::new("-").horizontal_alignment(iced::alignment::Horizontal::Center),
                )
                .width(Length::Fixed(20.0))
                .on_press(Message::DeleteRaid),
            );
        }

        let mut content = Column::new()
            .spacing(20)
            .align_items(Alignment::Start)
            .push(pick_row);

        let raids = &self.raids;

        if let Some(raid) = self
            .current_raid
            .clone()
            .map(|name| raids.get(&name))
            .flatten()
        {
            content = content.push(
                Row::new()
                    .spacing(20)
                    .push(Text::new(format!(
                        "{} / {} codes tried",
                        raid.tried_code_count, raid.remaining_code_count,
                    )))
                    .push(Text::new(format!("You: {} codes", self.codes_tried))),
            );

            let last_codes = match self.last_codes.len() {
                0 => String::from("(None)"),
                _ => self.last_codes.join(", "),
            };

            content =
                content.push(Row::new().push(Text::new(format!("Last codes: {}", last_codes))));
        } else {
            content = content.spacing(15).push(
                Row::new()
                    .spacing(20)
                    .push(
                        TextInput::new("Raid Name", &self.create_raid_name)
                            .on_input(Message::CreateRaidInputChanged)
                            .width(Length::Fixed(310.0))
                            .padding(5),
                    )
                    .push(
                        Button::new(
                            Text::new("+")
                                .horizontal_alignment(iced::alignment::Horizontal::Center),
                        )
                        .on_press(Message::CreateRaid)
                        .width(Length::Fixed(20.0)),
                    ),
            )
        }

        if !self.connected {
            content = Column::new().push(Text::new("Connecting to codera1d server..."));
        }

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top)
            .padding(20)
            .into()
    }
}

impl CodeRaid {
    async fn fetch_raids(client: Arc<Client>) -> Option<HashMap<String, PartialRaid>> {
        let res = client
            .get(&format!("{}/raids", ENDPOINT))
            .send()
            .await
            .ok()?;

        let res = res.json().await.ok()?;

        Some(res)
    }

    async fn reserve_codes(raid: String, client: Arc<Client>) -> Option<(String, CodeReservation)> {
        let res = client
            .post(&format!("{}/raids/{}/reserve_codes", ENDPOINT, raid))
            .send()
            .await
            .ok()?;

        let res = res.json().await.ok()?;

        Some((raid.to_owned(), res))
    }

    async fn try_code(raid: String, code: String, client: Arc<Client>) -> Result<()> {
        client
            .post(&format!("{}/raids/{}/try_code", ENDPOINT, raid))
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await?;

        Ok(())
    }

    async fn create_raid(raid: String, client: Arc<Client>) -> Result<()> {
        client
            .post(&format!("{}/raids/", ENDPOINT))
            .json(&serde_json::json!({ "name": raid }))
            .send()
            .await?;

        Ok(())
    }

    async fn delete_raid(raid: String, client: Arc<Client>) -> Result<()> {
        client
            .delete(&format!("{}/raids", ENDPOINT))
            .json(&serde_json::json!({ "name": raid }))
            .send()
            .await?;

        Ok(())
    }

    fn cleanup_state(&mut self) {
        if_chain! {
            if let Some(raid) = &self.current_raid;
            if let None = self.raids.get(raid);

            then {
                self.current_raid = None
            }
        }

        self.code_reservations
            .iter_mut()
            .for_each(|(_, raid_reservations)| {
                raid_reservations.drain_filter(|reservation| {
                    if reservation.expires_at < (Utc::now() + chrono::Duration::seconds(45)) {
                        return true;
                    }

                    if reservation.codes.len() == 0 {
                        return true;
                    }

                    false
                });
            });
    }
}

fn input_code(code: &str) {
    use inputbot::KeybdKey::*;

    let keys = code
        .chars()
        .map(|char| match char {
            '0' => Some(Numrow0Key),
            '1' => Some(Numrow1Key),
            '2' => Some(Numrow2Key),
            '3' => Some(Numrow3Key),
            '4' => Some(Numrow4Key),
            '5' => Some(Numrow5Key),
            '6' => Some(Numrow6Key),
            '7' => Some(Numrow7Key),
            '8' => Some(Numrow8Key),
            '9' => Some(Numrow9Key),
            _ => None,
        })
        .collect::<Vec<Option<KeybdKey>>>();

    std::thread::spawn(move || {
        let mutex = KEYBOARD_MUTEX.lock().unwrap();
        keys.iter().for_each(|key| {
            if let Some(key) = key {
                key.press();
                std::thread::sleep(Duration::from_millis(35));
                key.release();
                std::thread::sleep(Duration::from_millis(35));
            }
        });

        std::mem::drop(mutex);
    });
}
