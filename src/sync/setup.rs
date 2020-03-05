use crate::db::objects::ObjectConfig;
use crate::db::Db;
use crate::salesforce::objects::SObject;
use crate::salesforce::Salesforce;
use std::sync::Arc;
use std::sync::Mutex;

const ERR_OBJECT_NOT_FOUND: &str = "Object not found";
const ERR_CACHE_NOT_SETUP: &str = "Cache not setup";

#[derive(Default, Clone)]
struct SyncObjectCache {
    pub sf_objects: Option<Vec<SObject>>,
    pub db_objects: Option<Vec<ObjectConfig>>,
}

#[derive(Clone)]
pub struct Setup {
    salesforce: Arc<Salesforce>,
    db: Arc<Db>,
    cache: Arc<Mutex<SyncObjectCache>>,
}

impl Setup {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>) -> Setup {
        Setup {
            salesforce: salesforce,
            db: db,
            cache: Default::default(),
        }
    }

    pub fn list_salesforce_objects<F, T>(&self, print_func: F) -> Result<Vec<T>, String>
    where
        F: FnMut((u32, &String, &String, bool, bool, bool)) -> T,
    {
        let sf_objects = self.salesforce.get_objects()?;
        self.cache.lock().unwrap().sf_objects = Some(sf_objects);
        let mut i: u32 = 0;
        let result = self
            .cache
            .lock()
            .unwrap()
            .sf_objects
            .as_ref()
            .unwrap()
            .iter()
            .map(|obj| {
                i += 1;
                let synced = match self.db.get_object_config(&obj.name) {
                    Some(_config) => true,
                    None => false,
                };
                (
                    i,
                    &obj.name,
                    &obj.label,
                    obj.custom_setting,
                    obj.createable,
                    synced,
                )
            })
            .map(print_func)
            .collect::<Vec<_>>();
        Ok(result)
    }

    pub fn list_db_objects<F, T>(&self, print_func: F) -> Result<Vec<T>, String>
    where
        F: FnMut((u32, &String, u32, usize)) -> T,
    {
        let objects = self.db.get_selected_objects(-1)?;
        debug!("list_db_objects: objects len: {}", objects.len());
        self.cache.lock().unwrap().db_objects = Some(objects);
        let mut i: u32 = 0;
        let result = self
            .cache
            .lock()
            .unwrap()
            .db_objects
            .as_ref()
            .unwrap()
            .iter()
            .map(|obj| {
                i += 1;
                (i, &obj.name, obj.count, obj.fields.len())
            })
            .map(print_func)
            .collect::<Vec<_>>();
        Ok(result)
    }

    pub fn sf_object_exists(&self, index: usize) -> bool {
        let name: String;
        {
            let cache = self.cache.lock().unwrap();
            let item = cache
                .sf_objects
                .as_ref()
                .ok_or(ERR_CACHE_NOT_SETUP)
                .unwrap()
                .get(index - 1)
                .ok_or(ERR_OBJECT_NOT_FOUND)
                .unwrap();
            name = item.name.clone();
        }
        println!("{}", name);
        match self.db.get_object_config(&name) {
            Some(_config) => return true,
            None => return false,
        }
    }

    pub fn setup_sf_object<F>(
        &self,
        index: usize,
        setup_db_sync: bool,
        notify: F,
    ) -> Result<(String, u64), String>
    where
        F: Fn(&str, u64),
    {
        let name: String;
        {
            let cache = self.cache.lock().unwrap();
            let item = cache
                .sf_objects
                .as_ref()
                .ok_or(ERR_CACHE_NOT_SETUP)?
                .get(index - 1)
                .ok_or(ERR_OBJECT_NOT_FOUND)?;
            name = item.name.clone();
        }

        notify(&format!("Selected Object: {}", name), 0);
        let describe = self.salesforce.describe_object(&name)?;
        match self.db.create_object_table(&name, &describe.fields) {
            Err(err) => {
                notify(&format!("Error on Object: {} {}", name, err), 0);
                return Err(err.to_string());
            }
            Ok(_) => {}
        }
        self.db.save_config_data(&describe);
        if setup_db_sync {
            self.db.add_channel_trigger(&name);
        }
        let wrapper = self
            .salesforce
            .get_records_from_describe(&describe, &name)?;
        let mut row_count = 0;
        row_count += self.db.populate(&wrapper)?;
        notify(&format!("Sync started for {}", name), row_count);

        let mut next_wrapper_opt = self.salesforce.get_next_records(&describe, &wrapper);
        while let Some(next_wrapper) = next_wrapper_opt {
            row_count += self.db.populate(&next_wrapper)?;
            notify(&format!("Sync running for {}", name), row_count);
            if !next_wrapper.done {
                // println!("Next Path: {}", next_wrapper.next_url);
            } else {
                notify(&format!("Sync ended for {}", name), row_count);
            }
            next_wrapper_opt = self.salesforce.get_next_records(&describe, &next_wrapper);
        }
        Ok((name, row_count))
    }

    pub fn delete_db_object(&self, index: usize) -> Result<String, String> {
        let cache = self.cache.lock().unwrap();
        let db_objects = cache.db_objects.as_ref().ok_or(ERR_CACHE_NOT_SETUP)?;
        let obj = &db_objects.get(index - 1).ok_or(ERR_OBJECT_NOT_FOUND)?;
        self.db.destroy(obj.id, &obj.name);
        Ok(obj.name.clone())
    }

    pub fn update_db_object(&self, index: usize) {}

    pub fn salesforce(&self) -> Arc<Salesforce> {
        self.salesforce.clone()
    }
}
