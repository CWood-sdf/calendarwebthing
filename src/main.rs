use std::{path::Path, path::PathBuf, sync::Arc};

use rocket::fs::{relative, FileServer, NamedFile};
use rocket_dyn_templates::{context, Template};

pub mod assignment;
pub mod job;
pub mod manager;
pub mod network;

use network::*;

#[macro_use]
extern crate rocket;

#[derive(serde::Serialize, serde::Deserialize)]
struct PageData {
    page: String,
    manager: manager::ManagerData,
    htmx_request: bool,
}

#[get("/")]
fn index(req: Headers) -> Template {
    get_page("index".to_string(), req)
}

#[get("/<page>")]
fn get_page(page: String, headers: Headers) -> Template {
    println!("Getting page {}", page);
    let manager = manager::Manager::read_no_save();
    println!("Manager: {:?}", manager);
    let page_data = PageData {
        page: page.clone(),
        manager,
        htmx_request: headers.contains("hx-request".to_string()),
    };
    if !page_data.htmx_request {
        Template::render("layout", context! {page_data})
    } else {
        Template::render(page, context! {page_data})
    }
}
#[get("/js/<path..>")]
async fn get_js(path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/js/").join(path))
        .await
        .ok()
}
#[get("/<folder>/<page>")]
fn get_sub_page(folder: String, page: String, headers: Headers) -> Template {
    println!("{}/{}", folder, page);
    let manager = manager::Manager::read_no_save();
    let page_data = PageData {
        page: format!("{}/{}", folder, page),
        manager,
        htmx_request: headers.contains("hx-request".to_string()),
    };
    if !page_data.htmx_request {
        Template::render("layout", context! {page_data})
    } else {
        Template::render(page_data.page.clone(), context! {page_data})
    }
}

// #[launch]
#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shutdown = Arc::new(std::sync::Mutex::new(false));
    let thread_shutdown = shutdown.clone();
    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        loop {
            let mut manager = manager::Manager::from_save_file();
            manager.clear_past_due();
            rt.block_on(manager.run_jobs());
            // std::thread::sleep(std::time::Duration::from_secs(60));
            manager.break_lock();
            // println!("Waiting...");
            let mut sleep_time = 10;
            let sleep_duration = 1;
            // sleep for 5 minutes
            while sleep_time > 0 {
                std::thread::sleep(std::time::Duration::from_secs(sleep_duration));
                let val = thread_shutdown.lock().unwrap();
                // println!("Checking for shutdown {}...", *val);
                if *val {
                    println!("Exiting thread...");
                    return;
                }
                sleep_time -= sleep_duration;
            }
        }
    });
    _ = rocket::build()
        .attach(Template::custom(|engines| {
            engines.handlebars.set_strict_mode(true);
        }))
        .configure(rocket::Config::figment().merge(("port", 6969)))
        .mount("/", routes![index, get_page, get_sub_page, get_js])
        .mount("/assignments", assignment::get_routes())
        .mount("/jobs", job::get_routes())
        .mount("/", FileServer::from(relative!("static/")))
        .launch()
        .await?;

    println!("Shutting down...");
    {
        let mut val = shutdown.lock().unwrap();
        *val = true;
        println!("Waiting for thread to finish...");
        drop(val);
    }

    thread.join().unwrap();
    println!("Thread finished");
    Ok(())
}
