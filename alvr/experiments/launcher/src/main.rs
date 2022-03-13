use alvr_common::{lazy_static, parking_lot::Mutex, prelude::*};
use alvr_filesystem as afs;
use alvr_gui::{Dashboard, DashboardDataInterfce, DashboardEvent};
use alvr_server_data::ServerDataManager;
use alvr_session::{ClientConnectionDesc, EventSeverity, Raw, ServerEvent, SessionDesc};
use alvr_sockets::AudioDevicesList;
use iced::{
    executor,
    futures::{
        channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
        lock::Mutex as AMutex,
        stream::{self, BoxStream},
        SinkExt, StreamExt,
    },
    window::{self, Position},
    Application, Command, Element, Settings, Subscription,
};
use iced_native::subscription::Recipe;
use std::{
    any::TypeId,
    collections::HashSet,
    env,
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

lazy_static! {
    static ref SESSION: Arc<Mutex<SessionDesc>> = {
        let mut session = SessionDesc::default();
        session.client_connections.insert(
            "1234.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest 2".into(),
                manual_ips: HashSet::new(),
                trusted: false,
            },
        );
        session.client_connections.insert(
            "4321.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest".into(),
                manual_ips: HashSet::new(),
                trusted: true,
            },
        );
        session.client_connections.insert(
            "51423.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest 2".into(),
                manual_ips: HashSet::new(),
                trusted: true,
            },
        );

        Arc::new(Mutex::new(session))
    };
    static ref SESSION_MODIFIED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

struct EventsRecipe {
    receiver: Arc<AMutex<UnboundedReceiver<ServerEvent>>>,
}

impl<H: Hasher, E> Recipe<H, E> for EventsRecipe {
    type Output = ServerEvent;

    fn hash(&self, state: &mut H) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _: BoxStream<E>) -> BoxStream<ServerEvent> {
        let receiver = Arc::clone(&self.receiver);
        Box::pin(stream::unfold((), move |_| {
            let receiver = Arc::clone(&receiver);
            async move { Some((receiver.lock().await.next().await?, ())) }
        }))
    }
}

struct DashboardWindow {
    dashboard: Dashboard,
    data_manager: Arc<Mutex<ServerDataManager>>,
    dashboard_data_interface: DashboardDataInterfce,
    event_receiver: Arc<AMutex<UnboundedReceiver<ServerEvent>>>,
    should_exit: bool,
}

impl Application for DashboardWindow {
    type Executor = executor::Default;
    type Message = DashboardEvent;
    type Flags = ();

    fn new(_: ()) -> (Self, Command<DashboardEvent>) {
        let fs_layout = afs::filesystem_layout_from_launcher_exe(&env::current_exe().unwrap());

        let data_manager = Arc::new(Mutex::new(ServerDataManager::new(&fs_layout.session())));

        // debug. todo: remove
        data_manager.lock().session_mut().client_connections.insert(
            "1234.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest 2".into(),
                manual_ips: HashSet::new(),
                trusted: false,
            },
        );
        data_manager.lock().session_mut().client_connections.insert(
            "4321.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest".into(),
                manual_ips: HashSet::new(),
                trusted: true,
            },
        );
        data_manager.lock().session_mut().client_connections.insert(
            "51423.client.alvr".into(),
            ClientConnectionDesc {
                display_name: "Oculus Quest 2".into(),
                manual_ips: HashSet::new(),
                trusted: true,
            },
        );

        let (event_sender, event_receiver) = mpsc::unbounded();
        let event_sender = Arc::new(Mutex::new(event_sender));
        let event_receiver = Arc::new(AMutex::new(event_receiver));

        // debug. todo: remove
        pollster::block_on(event_sender.lock().send(ServerEvent::Raw(Raw {
            timestamp: "time1".into(),
            severity: EventSeverity::Info,
            content: "test1".into(),
        })))
        .ok();
        pollster::block_on(event_sender.lock().send(ServerEvent::Raw(Raw {
            timestamp: "time2".into(),
            severity: EventSeverity::Warning,
            content: "test2".into(),
        })))
        .ok();
        pollster::block_on(event_sender.lock().send(ServerEvent::Raw(Raw {
            timestamp: "time3".into(),
            severity: EventSeverity::Error,
            content: "test3".into(),
        })))
        .ok();
        pollster::block_on(event_sender.lock().send(ServerEvent::Raw(Raw {
            timestamp: "time4".into(),
            severity: EventSeverity::Debug,
            content: "test4".into(),
        })))
        .ok();

        let mut dashboard_data_interface = DashboardDataInterfce {
            set_session_cb: {
                let data_manager = Arc::clone(&data_manager);
                let event_sender = Arc::clone(&event_sender);
                Box::new(move |path, value| {
                    let mut data_manager = data_manager.lock();
                    data_manager.set_single_value(path, value).unwrap();

                    pollster::block_on(event_sender.lock().send(ServerEvent::Session(Box::new(
                        data_manager.session().clone(),
                    ))))
                    .ok();
                })
            },
            execute_script_cb: {
                let data_manager = Arc::clone(&data_manager);
                Box::new(move |code| {
                    let result = data_manager.lock().execute_script(code);

                    match result {
                        Ok(value) => Some(value),
                        Err(e) => {
                            error!("{e}");
                            None
                        }
                    }
                })
            },
            get_gpu_name_cb: {
                let data_manager = Arc::clone(&data_manager);
                Box::new(move || data_manager.lock().get_gpu_names())
            },
            get_audio_devices_list_cb: {
                let data_manager = Arc::clone(&data_manager);
                Box::new(move || {
                    data_manager
                        .lock()
                        .get_audio_devices_list()
                        .unwrap_or_else(|e| {
                            error!("{e}");
                            AudioDevicesList {
                                output: vec![],
                                input: vec![],
                            }
                        })
                })
            },
        };

        let mut dashboard = Dashboard::new();
        dashboard.update(
            DashboardEvent::ServerEvent(ServerEvent::Session(Box::new(
                data_manager.lock().session().clone(),
            ))),
            &mut dashboard_data_interface,
        );

        (
            Self {
                dashboard,
                data_manager,
                dashboard_data_interface,
                event_receiver,
                should_exit: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "ALVR Dashboard".into()
    }

    fn update(&mut self, event: DashboardEvent) -> Command<DashboardEvent> {
        if let DashboardEvent::ServerEvent(ServerEvent::ServerQuitting) = event {
            self.should_exit = true;
        }

        self.dashboard
            .update(event, &mut self.dashboard_data_interface);

        Command::none()
    }

    fn view(&mut self) -> Element<DashboardEvent> {
        self.dashboard.view()
    }

    fn subscription(&self) -> Subscription<DashboardEvent> {
        Subscription::from_recipe(EventsRecipe {
            receiver: Arc::clone(&self.event_receiver),
        })
        .map(DashboardEvent::ServerEvent)
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }
}

fn main() {
    DashboardWindow::run(Settings {
        id: None,
        window: window::Settings {
            size: (800, 600),
            position: Position::Centered,
            icon: None, // todo
            ..Default::default()
        },
        flags: (),
        default_font: None,
        default_text_size: 16,
        text_multithreading: false,
        antialiasing: false,
        exit_on_close_request: true,
        try_opengles_first: false,
    })
    .unwrap();
    // let data_manager = Session
    // let dashboard = Arc::new(Dashboard::new());

    // dashboard.report_event(ServerEvent::Raw(Raw {
    //     timestamp: "time1".into(),
    //     severity: EventSeverity::Info,
    //     content: "test1".into(),
    // }));
    // dashboard.report_event(ServerEvent::Raw(Raw {
    //     timestamp: "time2".into(),
    //     severity: EventSeverity::Warning,
    //     content: "test2".into(),
    // }));
    // dashboard.report_event(ServerEvent::Raw(Raw {
    //     timestamp: "time3".into(),
    //     severity: EventSeverity::Error,
    //     content: "test3".into(),
    // }));
    // dashboard.report_event(ServerEvent::Raw(Raw {
    //     timestamp: "time4".into(),
    //     severity: EventSeverity::Debug,
    //     content: "test4".into(),
    // }));

    // let mut engine = rhai::Engine::new();

    // let mut scope = rhai::Scope::new();
    // engine.register_fn("load_session", load_session_dyn);
    // engine.register_fn("store_session", store_session);
    // engine.register_fn("add_client", add_client);
    // engine.register_fn("trust_client", trust_client);
    // engine.register_fn("remove_client", remove_client);

    // dashboard.run(
    //     load_session(),
    //     {
    //         let dashboard = Arc::clone(&dashboard);
    //         Box::new(move |path, value| {
    //             let mut session_json =
    //                 serde_json::to_value(SESSION.lock().unwrap().clone()).unwrap();

    //             let mut session_ref = &mut session_json;
    //             for segment in path {
    //                 session_ref = match segment {
    //                     PathSegment::Name(name) => &mut session_ref[name],
    //                     PathSegment::Index(index) => &mut session_ref[index],
    //                 };
    //             }

    //             *session_ref = serde_json::from_str(&value).unwrap();

    //             // session_json has been updated
    //             *SESSION.lock().unwrap() = serde_json::from_value(session_json).unwrap();

    //             dashboard.report_event(ServerEvent::Session(Box::new(
    //                 SESSION.lock().unwrap().clone(),
    //             )))
    //         })
    //     },
    //     {
    //         let dashboard = Arc::clone(&dashboard);
    //         Box::new(move |command| {
    //             let res = engine
    //                 .eval_with_scope::<rhai::Dynamic>(&mut scope, &command)
    //                 .map(|d| d.to_string())
    //                 .map_err(|e| e.to_string());

    //             if SESSION_MODIFIED.load(Ordering::Relaxed) {
    //                 dashboard.report_event(ServerEvent::Session(Box::new(
    //                     SESSION.lock().unwrap().clone(),
    //                 )))
    //             }

    //             res
    //         })
    //     },
    // );
}
