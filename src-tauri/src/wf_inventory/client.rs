use crate::{
    app::Settings,
    wf_inventory::{inv_sources::*, modules::*, WarframeRootObject},
};
use std::sync::{Arc, Mutex, OnceLock};

use crate::wf_inventory::snapshot::{root_fingerprint, InventorySnapshot};
use utils::*;

pub struct WFInventoryState {
    source: Mutex<WFInventorySource>,
    root: Arc<Mutex<WarframeRootObject>>,
    item_module: OnceLock<Arc<ItemModule>>,
    riven_module: OnceLock<Arc<RivenModule>>,
    syndicate_module: OnceLock<Arc<SyndicateModule>>,
    sets_module: OnceLock<Arc<SetsModule>>,
    /// The derived rows and the fingerprint of the inventory they came from.
    snapshot: Mutex<Option<(u64, Arc<InventorySnapshot>)>>,
}

impl WFInventoryState {
    pub fn new(settings: &Settings) -> Arc<Self> {
        let source = settings.wf_inventory.source.clone();
        info(
            "WFInventoryState:New",
            format!("Starting inventory source: {}", source),
            &LoggerOptions::default(),
        );
        let state = Arc::new(Self {
            source: Mutex::new(source),
            root: Arc::new(Mutex::new(WarframeRootObject::default())),
            item_module: OnceLock::new(),
            riven_module: OnceLock::new(),
            syndicate_module: OnceLock::new(),
            sets_module: OnceLock::new(),
            snapshot: Mutex::new(None),
        });

        // Start the source (initial load + watcher for alecaframe)
        state.source.lock().unwrap().start(&state.root);
        state.init_modules();
        state
    }

    /// The derived rows for every tab, rebuilt only when the inventory
    /// itself changes.
    ///
    /// Building a row needs a tradable-items lookup, which takes the same
    /// mutex the live scraper holds and clones the item. Doing that per
    /// request made a sort click pay for the whole inventory to render one
    /// page. The fingerprint is hashed under the root lock without cloning
    /// it, which is cheap next to the work it skips.
    pub fn rows(&self) -> Result<Arc<InventorySnapshot>, Error> {
        let fingerprint = root_fingerprint(&self.root.lock().unwrap());
        if let Some((cached, snapshot)) = self.snapshot.lock().unwrap().as_ref() {
            if *cached == fingerprint {
                return Ok(snapshot.clone());
            }
        }

        // Built outside the snapshot lock: it is slow, and a second caller
        // arriving meanwhile should wait on nothing worse than doing the same
        // work twice.
        let built = Arc::new(InventorySnapshot::build(&self.get_root())?);
        *self.snapshot.lock().unwrap() = Some((fingerprint, built.clone()));
        Ok(built)
    }

    pub fn get_root(&self) -> WarframeRootObject {
        let root = self.root.lock().unwrap().clone();
        root
    }

    pub fn update(&self) -> Result<(), Error> {
        self.source.lock().unwrap().update(&self.root)
    }

    pub fn set_source(&self, source: WFInventorySource) {
        self.source.lock().unwrap().stop();
        *self.source.lock().unwrap() = source;
        self.source.lock().unwrap().start(&self.root);
    }

    fn init_modules(self: &Arc<Self>) {
        self.item_module
            .get_or_init(|| ItemModule::new(self.clone()));
        self.riven_module
            .get_or_init(|| RivenModule::new(self.clone()));
        self.syndicate_module
            .get_or_init(|| SyndicateModule::new(self.clone()));
        self.sets_module
            .get_or_init(|| SetsModule::new(self.clone()));
    }

    pub fn item(&self) -> Arc<ItemModule> {
        self.item_module
            .get()
            .expect("ItemModule not initialized")
            .clone()
    }

    pub fn riven(&self) -> Arc<RivenModule> {
        self.riven_module
            .get()
            .expect("RivenModule not initialized")
            .clone()
    }
    pub fn syndicate(&self) -> Arc<SyndicateModule> {
        self.syndicate_module
            .get()
            .expect("SyndicateModule not initialized")
            .clone()
    }
    pub fn sets(&self) -> Arc<SetsModule> {
        self.sets_module
            .get()
            .expect("SetsModule not initialized")
            .clone()
    }
}
