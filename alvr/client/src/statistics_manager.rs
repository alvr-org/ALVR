use alvr_common::data::ClientStatistics;

pub struct StatisticsManager {}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn report_frame_to_be_decoded(&mut self, idx: i64) {
        todo!()
    }

    pub fn report_decoded_frame(&mut self, idx: i64) {
        todo!()
    }

    pub fn get(&mut self) -> ClientStatistics {
        todo!()
    }
}
