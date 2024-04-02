use crate::{actions, InstallationInfo, Progress, ReleaseChannelsInfo, UiMessage, WorkerMessage};
use eframe::{
    egui::{
        self, Button, CentralPanel, ComboBox, Context, Frame, Grid, Layout, ProgressBar, RichText,
        ViewportCommand, Window,
    },
    emath::{Align, Align2},
    epaint::Color32,
};
use std::{
    mem,
    sync::mpsc::{Receiver, Sender},
};

enum State {
    Default,
    Installing(Progress),
    Error(String),
}

#[derive(Default)]
enum Popup {
    #[default]
    None,
    DeleteInstallation(String),
    EditVersion(String),
    Version(VersionPopup),
}

#[derive(Clone, PartialEq, Eq)]
enum ReleaseChannelType {
    Stable,
    Nightly,
}

#[derive(Clone, PartialEq, Eq)]
struct Version {
    version: String,
    release_channel: ReleaseChannelType,
}

struct VersionPopup {
    version: Version,
}

pub struct Launcher {
    worker_message_receiver: Receiver<WorkerMessage>,
    ui_message_sender: Sender<UiMessage>,
    state: State,
    release_channels_info: Option<ReleaseChannelsInfo>,
    installations: Vec<InstallationInfo>,
    popup: Popup,
}

impl Launcher {
    pub fn new(
        cc: &eframe::CreationContext,
        worker_message_receiver: Receiver<WorkerMessage>,
        ui_message_sender: Sender<UiMessage>,
    ) -> Self {
        alvr_gui_common::theme::set_theme(&cc.egui_ctx);

        Self {
            worker_message_receiver,
            ui_message_sender,
            state: State::Default,
            release_channels_info: None,
            installations: actions::get_installations(),
            popup: Popup::None,
        }
    }

    fn version_popup(&mut self, ctx: &Context, mut version_popup: VersionPopup) -> Popup {
        Window::new("Add version")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                // Safety: unwrap is safe because the "Add release" button is available after populating the release_channels_info.
                let release_channels_info = self.release_channels_info.as_ref().unwrap();
                let (channel, version_str, versions): (&str, String, Vec<Version>) =
                    match version_popup.version.release_channel.clone() {
                        ReleaseChannelType::Stable => (
                            "Stable",
                            version_popup.version.version.clone(),
                            release_channels_info
                                .stable
                                .iter()
                                .map(|release| Version {
                                    version: release.version.clone(),
                                    release_channel: ReleaseChannelType::Stable,
                                })
                                .collect(),
                        ),
                        ReleaseChannelType::Nightly => (
                            "Nightly",
                            version_popup.version.version.clone(),
                            release_channels_info
                                .nightly
                                .iter()
                                .map(|release| Version {
                                    version: release.version.clone(),
                                    release_channel: ReleaseChannelType::Nightly,
                                })
                                .collect(),
                        ),
                    };
                Grid::new("add-version-grid").num_columns(2).show(ui, |ui| {
                    ui.label("Channel");

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ComboBox::from_id_source("channel")
                            .selected_text(channel)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut version_popup.version,
                                    Version {
                                        version: self
                                            .release_channels_info
                                            .as_ref()
                                            .unwrap()
                                            .stable[0]
                                            .version
                                            .clone(),
                                        release_channel: ReleaseChannelType::Stable,
                                    },
                                    "Stable",
                                );
                                ui.selectable_value(
                                    &mut version_popup.version,
                                    Version {
                                        version: self
                                            .release_channels_info
                                            .as_ref()
                                            .unwrap()
                                            .nightly[0]
                                            .version
                                            .clone(),
                                        release_channel: ReleaseChannelType::Nightly,
                                    },
                                    "Nightly",
                                );
                            })
                    });
                    ui.end_row();

                    ui.label("Version");
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ComboBox::from_id_source("version")
                            .selected_text(version_str)
                            .show_ui(ui, |ui| {
                                for version in versions {
                                    ui.selectable_value(
                                        &mut version_popup.version,
                                        version.clone(),
                                        version.version,
                                    );
                                }
                            })
                    });
                    ui.end_row();
                });
                ui.columns(2, |ui| {
                    if ui[0].button("Cancel").clicked() {
                        return Popup::None;
                    }

                    if ui[1].button("Install").clicked() {
                        self.ui_message_sender
                            .send(UiMessage::InstallServer(
                                match &version_popup.version.release_channel {
                                    ReleaseChannelType::Stable => release_channels_info
                                        .stable
                                        .iter()
                                        .find(|release| {
                                            release.version == version_popup.version.version
                                        })
                                        .unwrap()
                                        .clone(),
                                    ReleaseChannelType::Nightly => release_channels_info
                                        .nightly
                                        .iter()
                                        .find(|release| {
                                            release.version == version_popup.version.version
                                        })
                                        .unwrap()
                                        .clone(),
                                },
                            ))
                            .unwrap();
                        return Popup::None;
                    }

                    Popup::Version(version_popup)
                })
            })
            .unwrap()
            .inner
            .unwrap()
    }

    fn edit_popup(&self, ctx: &Context, version: String) -> Popup {
        Window::new("Edit version")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                    if ui.button("Delete version").clicked() {
                        return Popup::DeleteInstallation(version);
                    };
                    if ui.button("Close").clicked() {
                        return Popup::None;
                    }

                    Popup::EditVersion(version)
                })
                .inner
            })
            .unwrap()
            .inner
            .unwrap()
    }

    fn delete_popup(&mut self, ctx: &Context, version: String) -> Popup {
        Window::new("Are you sure?")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(format!("This will permanently delete version {}", version));
                });
                ui.columns(2, |ui| {
                    if ui[0].button("Cancel").clicked() {
                        return Popup::None;
                    }
                    if ui[1].button("Delete version").clicked() {
                        if let Err(e) = actions::delete_installation(&version) {
                            self.state = State::Error(format!("Failed to delete version: {e}"));
                        }

                        self.installations = actions::get_installations();

                        return Popup::None;
                    }
                    Popup::DeleteInstallation(version)
                })
            })
            .unwrap()
            .inner
            .unwrap()
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        while let Ok(msg) = self.worker_message_receiver.try_recv() {
            match msg {
                WorkerMessage::ReleaseChannelsInfo(data) => self.release_channels_info = Some(data),
                WorkerMessage::ProgressUpdate(progress) => {
                    self.state = State::Installing(progress);
                }
                WorkerMessage::Done => {
                    // Refresh installations
                    self.installations = actions::get_installations();
                    self.state = State::Default;
                }
                WorkerMessage::Error(e) => self.state = State::Error(e),
            }
        }

        CentralPanel::default().show(ctx, |ui| match &self.state {
            State::Default => {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(RichText::new("ALVR Launcher").size(25.0).strong());
                    ui.label(match &self.release_channels_info {
                        Some(data) => format!("Latest stable release: {}", data.stable[0].version),
                        None => "Fetching latest release...".to_string(),
                    });

                    for installation in &self.installations {
                        let path = actions::installations_dir().join(&installation.version);

                        Frame::group(ui.style())
                            .fill(alvr_gui_common::theme::SECTION_BG)
                            .inner_margin(egui::vec2(10.0, 5.0))
                            .show(ui, |ui| {
                                Grid::new(&installation.version)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label(&installation.version);
                                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                            if ui.button("Edit").clicked() {
                                                self.popup = Popup::EditVersion(
                                                    installation.version.clone(),
                                                );
                                            }

                                            if ui.button("Open directory").clicked() {
                                                open::that_in_background(path);
                                            }

                                            let release_info = self
                                                .release_channels_info
                                                .as_ref()
                                                .and_then(|info| {
                                                    actions::get_release(
                                                        info,
                                                        &installation.version,
                                                    )
                                                });
                                            if ui
                                                .add_enabled(
                                                    release_info.is_some()
                                                        || installation.is_apk_downloaded,
                                                    Button::new("Install APK"),
                                                )
                                                .clicked()
                                            {
                                                if let Some(release_info) = release_info {
                                                    self.ui_message_sender
                                                        .send(UiMessage::InstallClient(
                                                            release_info,
                                                        ))
                                                        .unwrap();
                                                } else {
                                                    self.state = State::Error(
                                                        "Failed to get release info".to_string(),
                                                    );
                                                }
                                            };

                                            if ui.button("Launch").clicked() {
                                                match actions::launch_dashboard(
                                                    &installation.version,
                                                ) {
                                                    Ok(()) => {
                                                        self.ui_message_sender
                                                            .send(UiMessage::Quit)
                                                            .unwrap();
                                                        ctx.send_viewport_cmd(
                                                            ViewportCommand::Close,
                                                        );
                                                    }
                                                    Err(e) => {
                                                        self.state = State::Error(e.to_string());
                                                    }
                                                }
                                            }
                                        })
                                    })
                            });
                    }

                    if ui
                        .add_enabled(
                            self.release_channels_info.is_some(),
                            Button::new("Add version"),
                        )
                        .clicked()
                    {
                        self.popup = Popup::Version(VersionPopup {
                            version: Version {
                                version: self.release_channels_info.as_ref().unwrap().stable[0]
                                    .version
                                    .clone(),
                                release_channel: ReleaseChannelType::Stable,
                            },
                        });
                    }

                    let popup = match mem::take(&mut self.popup) {
                        Popup::Version(version_popup) => self.version_popup(ctx, version_popup),
                        Popup::EditVersion(version) => self.edit_popup(ctx, version),
                        Popup::DeleteInstallation(version) => self.delete_popup(ctx, version),
                        Popup::None => Popup::None,
                    };
                    self.popup = popup;
                });
            }
            State::Installing(progress) => {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(&progress.message);
                    ui.add(ProgressBar::new(progress.progress).animate(true));
                });
            }
            State::Error(e) => {
                let e = e.clone(); // Avoid borrowing issues with the closure for the layout
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.colored_label(Color32::LIGHT_RED, "Error!");
                    ui.label(e);

                    if ui.button("Close").clicked() {
                        self.state = State::Default;
                    }
                });
            }
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            self.ui_message_sender.send(UiMessage::Quit).unwrap();
        }
    }
}
