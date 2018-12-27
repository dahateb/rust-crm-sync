use db::objects::ObjectConfig;
use db::Db;
use salesforce::objects::SObject;
use salesforce::Salesforce;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex;

const ERR_OBJECT_NOT_FOUND: &str = "Object not found";
const ERR_CACHE_NOT_SETUP: &str = "Cache not setup";

#[derive(Default)]
struct SyncObjectCache {
    pub sf_objects: Option<Vec<SObject>>,
    pub db_objects: Option<Vec<ObjectConfig>>,
}

pub struct Setup {
    salesforce: Arc<Salesforce>,
    db: Arc<Db>,
    cache: Mutex<SyncObjectCache>,
}

impl Setup {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>) -> Setup {
        Setup {
            salesforce: salesforce,
            db: db,
            cache: Default::default(),
        }
    }

    pub fn list_salesforce_objects<F>(&self, print_func: F) -> Result<usize, String>
    where
        F: FnMut(&(u32, &String, bool)),
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
                (i, &obj.name, obj.createable)
            })
            .inspect(print_func)
            .count();
        Ok(result)
    }

    pub fn list_db_objects<F>(&self, print_func: F) -> Result<usize, String>
    where
        F: FnMut(&(u32, &String, u32)),
    {
        let objects = self.db.get_selected_objects(-1)?;
        self.cache.lock().unwrap().db_objects = Some(objects);
        let mut i: u32 = 0;
        let count = self
            .cache
            .lock()
            .unwrap()
            .db_objects
            .as_ref()
            .unwrap()
            .iter()
            .map(|obj| {
                i += 1;
                (i, &obj.name, obj.count)
            })
            .inspect(print_func)
            .count();
        Ok(count)
    }

    pub fn setup_sf_object(
        &self,
        index: usize,
        setup_db_sync: bool,
    ) -> Result<(String, u64), String> {
        let cache = &self.cache.lock().unwrap();
        let item = &cache
            .sf_objects
            .as_ref()
            .ok_or(ERR_CACHE_NOT_SETUP)?
            .get(index - 1)
            .ok_or(ERR_OBJECT_NOT_FOUND)?;
        // println!("selected object: {}", item.name);
        let describe = self.salesforce.describe_object(&item.name)?;
        self.db.save_config_data(&describe);
        self.db.create_object_table(&item.name, &describe.fields);
        if setup_db_sync {
            self.db.add_channel_trigger(&item.name);
        }
        let wrapper = self
            .salesforce
            .get_records_from_describe(&describe, &item.name)?;
        let mut row_count = 0;
        row_count += self.db.populate(&wrapper)?;
        print!(".");
        io::stdout().flush().unwrap();
        // println!("Synched {} rows", row_count);
        let mut next_wrapper_opt = self.salesforce.get_next_records(&describe, &wrapper);
        while let Some(next_wrapper) = next_wrapper_opt {
            row_count += self.db.populate(&next_wrapper)?;
            print!(".");
            io::stdout().flush().unwrap();
            // println!("Synched {} rows", row_count);
            if !next_wrapper.done {
                // println!("Next Path: {}", next_wrapper.next_url);
            } else {
                println!("");
                // println!("Done: {} rows", row_count);
            }
            next_wrapper_opt = self.salesforce.get_next_records(&describe, &next_wrapper);
        }
        Ok((item.name.clone(), row_count))
    }

    pub fn delete_db_object(&self, index: usize) -> Result<String, String> {
        let cache = &self.cache.lock().unwrap();
        let db_objects = cache.db_objects.as_ref().ok_or(ERR_CACHE_NOT_SETUP)?;
        let obj = &db_objects.get(index - 1).ok_or(ERR_OBJECT_NOT_FOUND)?;
        self.db.destroy(obj.id, &obj.name);
        Ok(obj.name.clone())
    }
}
