use alvr_sockets::DashboardRequest;
use eframe::egui::Ui;

pub fn debug_tab_ui(ui: &mut Ui) -> Option<DashboardRequest> {
    let mut request = None;

    ui.columns(4, |ui| {
        if ui[0].button("Capture frame").clicked() {
            request = Some(DashboardRequest::CaptureFrame);
        }

        if ui[1].button("Insert IDR").clicked() {
            request = Some(DashboardRequest::InsertIdr);
        }

        if ui[2].button("Start recording").clicked() {
            request = Some(DashboardRequest::StartRecording);
        }

        if ui[3].button("Stop recording").clicked() {
            request = Some(DashboardRequest::StopRecording);
        }
    });

    request
}
