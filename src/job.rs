use rocket::form::Form;
use rocket_dyn_templates::context;
use rocket_dyn_templates::Template;

use crate::network::*;

use crate::manager;
#[derive(serde::Serialize, serde::Deserialize)]
struct JobPageData {
    page: String,
    manager: manager::Manager,
    job: manager::Job,
    htmx_request: bool,
}
#[derive(FromForm)]
struct NewJob {
    name: String,
    path: String,
    sync_to_google: bool,
    sync_interval: u64,
}
#[delete("/<name>/delete")]
fn delete_job(name: String) -> rocket::response::Redirect {
    let mut manager = manager::Manager::from_save_file();
    manager.jobs.retain(|job| job.name != name);
    manager.save().unwrap();
    rocket::response::Redirect::to("/jobs")
}

#[post("/edit", data = "<job>")]
fn edit_job(job: Form<NewJob>) -> rocket::response::Redirect {
    let mut manager = manager::Manager::from_save_file();
    if !manager.jobs.iter().any(|j| j.name == job.name) {
        return rocket::response::Redirect::to(format!("/{}", job.name));
    }
    let current_job = manager
        .jobs
        .iter_mut()
        .find(|j| j.name == job.name)
        .unwrap();
    current_job.path = manager::Job::fix_home(job.path.clone());
    current_job.sync_to_google = job.sync_to_google;
    current_job.sync_interval = job.sync_interval;
    current_job.next_sync = current_job.last_sync + current_job.sync_interval;
    manager.save().unwrap();
    rocket::response::Redirect::to(format!("/jobs"))
}
#[post("/new", data = "<job>")]
fn new_job(job: Form<NewJob>) -> rocket::response::Redirect {
    let mut manager = manager::Manager::from_save_file();
    if manager.jobs.iter().any(|j| j.name == job.name) {
        return rocket::response::Redirect::to(format!("/jobs"));
    }
    let job = manager::Job::new(
        job.name.clone(),
        format!("~/{}", job.path),
        job.sync_to_google,
        job.sync_interval,
    );
    manager.add_job(job);
    manager.save().unwrap();
    rocket::response::Redirect::to(format!("/jobs"))
}

#[post("/<name>/run")]
fn run_job(name: String) -> &'static str {
    let mut manager = manager::Manager::from_save_file();
    manager
        .jobs
        .iter_mut()
        .find(|job| job.name == name)
        .unwrap()
        .next_sync = 0;
    manager.save().unwrap();
    println!("Request sent to run job {}", name);
    "Request sent to run job"
}
#[get("/<id>/<name>")]
fn get_page_job(
    id: String,
    name: String,
    headers: Headers,
) -> Result<Template, rocket::response::status::NotFound<String>> {
    let manager = manager::Manager::from_save_file();
    let page_data = JobPageData {
        page: format!("jobs/{}", name).to_string(),
        job: match manager.jobs.iter().find(|job| job.name == id) {
            Some(job) => job.clone(),
            None => {
                return Err(rocket::response::status::NotFound(
                    "Job not found".to_string(),
                ))
            }
        },
        manager,
        htmx_request: headers.contains("hx-request".to_string()),
    };
    if headers.contains("hx-request".to_string()) {
        let page = page_data.page.clone();
        println!("Rendering page: {}", page);
        Ok(Template::render(page, context! {page_data}))
    } else {
        Ok(Template::render("layout", context! {page_data}))
    }
}
#[post("/new_cli/<name>/<path>/<sync_to_google>/<sync_interval>")]
fn new_job_cli(
    name: String,
    path: String,
    sync_to_google: bool,
    sync_interval: u64,
) -> &'static str {
    let mut manager = manager::Manager::from_save_file();
    let job = manager::Job::new(name, format!("~/{}", path), sync_to_google, sync_interval);
    manager.add_job(job);
    manager.save().unwrap();
    "Job added"
}

pub fn get_routes() -> Vec<rocket::Route> {
    routes![
        edit_job,
        delete_job,
        new_job,
        new_job_cli,
        get_page_job,
        run_job
    ]
}
