use alvr_common::data::ClientStatistics;

pub struct StatisticsManager {}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn reportFrameToBeDecoded(&mut self, idx: i64) {
        todo!()
    }

    pub fn reportDecodedFrame(&mut self, idx: i64) {
        todo!()
    }

    pub fn get(&mut self) -> ClientStatistics {
        todo!()
    }
}
