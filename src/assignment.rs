pub(crate) use crate::manager;
use crate::network::*;
use rocket::time::format_description;
use rocket::Route;
use rocket_dyn_templates::{context, Template};
#[derive(serde::Serialize, serde::Deserialize)]
struct AssignmentPageData {
    page: String,
    manager: manager::ManagerData,
    job: Option<manager::Job>,
    assignment: manager::Assignment,
    htmx_request: bool,
}

#[derive(FromForm, Debug)]
struct NewAssignment {
    name: String,
    due: String,
    course: String,
}

#[post("/new", data = "<new_assignment>")]
fn new_assignment(
    new_assignment: rocket::form::Form<NewAssignment>,
) -> Result<rocket::response::Redirect, rocket::response::status::BadRequest<String>> {
    // println!("{:?}", new_assignment);
    let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]").unwrap();

    let due = new_assignment.due.replace("T", " ");
    let date = rocket::time::PrimitiveDateTime::parse(&due, &format);
    let date = match date {
        Ok(v) => v,
        Err(e) => {
            println!("{:?}, {}", e, due);
            return Err(rocket::response::status::BadRequest(
                "Invalid date".to_string(),
            ));
        }
    };
    let d = date.assume_utc().unix_timestamp();
    // println!("{}", d);
    // Ok("sdf".to_string())
    let mut manager = manager::Manager::from_save_file();
    let assignment = manager::Assignment::new(
        new_assignment.course.clone(),
        d as u64,
        new_assignment.name.clone(),
        "manual".to_string(),
    );
    manager.add_assignment(assignment);
    match manager.save() {
        Ok(_) => {}
        Err(e) => {
            return Err(rocket::response::status::BadRequest(e.to_string()));
        }
    }
    manager.break_lock();
    return Ok(rocket::response::Redirect::to("/assignments"));
    //     }
}
#[post("/edit", data = "<assignment>")]
fn edit_job(assignment: rocket::form::Form<NewAssignment>) -> rocket::response::Redirect {
    let mut manager = manager::Manager::from_save_file();
    if !manager
        .data
        .assignments
        .iter()
        .any(|j| j.name == assignment.name)
    {
        return rocket::response::Redirect::to(format!(
            "/{}",
            manager::Assignment::get_link_name(assignment.name.clone())
        ));
    }
    let current_assignment = manager
        .data
        .assignments
        .iter_mut()
        .find(|j| j.name == assignment.name)
        .unwrap();
    let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]").unwrap();
    let due = assignment.due.replace("T", " ");
    let date = rocket::time::PrimitiveDateTime::parse(&due, &format);
    let date = match date {
        Ok(v) => v,
        Err(e) => {
            println!("{:?}, {}", e, due);
            return rocket::response::Redirect::to(format!(
                "/{}",
                manager::Assignment::get_link_name(assignment.name.clone())
            ));
        }
    };
    let d = date.assume_utc().unix_timestamp();
    current_assignment.due = d as u64;
    current_assignment.course = assignment.course.clone();
    manager.save().unwrap();
    rocket::response::Redirect::to(format!("/assignments"))
}
#[delete("/<name>/delete")]
fn delete_assignment(name: String) -> rocket::response::Redirect {
    let mut manager = manager::Manager::from_save_file();
    manager
        .data
        .assignments
        .retain(|assignment| assignment.link_name != name);
    manager.save().unwrap();
    rocket::response::Redirect::to("/assignments")
}

#[get("/<id>/<name>")]
fn get_page_assignment(
    id: String,
    name: String,
    headers: Headers,
) -> Result<Template, rocket::response::status::NotFound<String>> {
    println!("{}", name);
    let manager = manager::Manager::read_no_save();
    let assignment = match manager.get_assignment_from_link_name(id) {
        Some(assignment) => assignment,
        None => {
            return Err(rocket::response::status::NotFound(
                "Assignment not found".to_string(),
            ))
        }
    };
    let page_data = AssignmentPageData {
        page: format!("assignments/{}", name).to_string(),
        assignment: assignment.clone(),
        job: match manager
            .jobs
            .iter()
            .find(|job| job.name == assignment.clone().job_name.unwrap_or("".to_string()))
        {
            Some(job) => Some(job.clone()),
            None => None,
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

pub fn get_routes() -> Vec<Route> {
    routes![
        get_page_assignment,
        new_assignment,
        delete_assignment,
        edit_job
    ]
}
