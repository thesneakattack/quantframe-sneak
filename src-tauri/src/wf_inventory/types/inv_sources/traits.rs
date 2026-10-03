use crate::wf_inventory::WarframeRootObject;
use std::sync::{Arc, Mutex};
use utils::*;

pub trait InventorySource {
    fn update(&self, root: &Arc<Mutex<WarframeRootObject>>) -> Result<(), Error>;

    fn start(&self, _root: &Arc<Mutex<WarframeRootObject>>) {}

    fn stop(&self) {}

    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
