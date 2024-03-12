use std::{cmp::Ordering, path::PathBuf};

use serde::{Deserialize, Serialize};

use tokio::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptAssignment {
    pub course: String,
    pub due: u64,
    pub name: String,
}
impl ScriptAssignment {
    pub fn to_assignment(&self, job_name: String) -> Assignment {
        Assignment::new(self.course.clone(), self.due, self.name.clone(), job_name)
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub name: String,
    pub path: PathBuf,
    pub sync_to_google: bool,
    pub next_sync: u64,
    pub last_sync: u64,
    pub sync_interval: u64,
}

impl Job {
    pub fn fix_home(path: String) -> PathBuf {
        PathBuf::from(path.replace("~", home::home_dir().unwrap().to_str().unwrap()))
    }
    pub fn new(name: String, path: String, sync_to_google: bool, sync_interval: u64) -> Self {
        Self {
            name,
            path: Self::fix_home(path),
            sync_to_google,
            next_sync: 0,
            last_sync: 0,
            sync_interval,
        }
    }
    fn sync_due(&self) -> bool {
        self.next_sync * 1000
            < std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
    }

    async fn sync(&mut self) -> Result<Vec<Assignment>, Box<dyn std::error::Error>> {
        self.last_sync = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.next_sync = self.last_sync + self.sync_interval;
        println!("Starting sync for {}", self.name);

        let output = match Command::new("node")
            .arg("index.js")
            .current_dir(&self.path)
            .output()
            .await
        {
            Ok(output) => output,
            Err(e) => {
                println!("Error syncing {}", self.name);
                return Err(Box::new(e));
            }
        };

        println!("Done syncing {}", self.name);
        if output.status.success() {
            println!("Synced {}", self.name);
            println!("{}", String::from_utf8(output.stdout.clone())?);
            let stdout = String::from_utf8(output.stdout)?;
            let assignments: Vec<ScriptAssignment> = match serde_json::from_str(&stdout) {
                Ok(v) => v,
                _ => vec![],
            };
            let assignments = assignments
                .into_iter()
                .map(|a| a.to_assignment(self.name.clone()))
                .collect();
            return Ok(assignments);
        }

        Ok(Vec::new())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Assignment {
    pub course: String,
    pub due: u64,
    pub name: String,
    pub job_name: Option<String>,
    pub synced: Option<bool>,
    pub link_name: String,
    pub done: bool,
}

impl PartialEq for Assignment {
    fn eq(&self, other: &Self) -> bool {
        self.course == other.course && self.name == other.name && self.job_name == other.job_name
    }
}

impl Assignment {
    pub fn get_link_name(name: String) -> String {
        name.replace(" ", "-").replace("/", "-").replace(".", "-")
    }
    pub fn new(course: String, due: u64, name: String, job_name: String) -> Self {
        Self {
            course,
            due,
            name: name.clone(),
            job_name: Some(job_name),
            synced: None,
            link_name: Self::get_link_name(name),
            done: false,
        }
    }
    pub fn past_due(&self) -> bool {
        self.due * 1000
            < std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
    }
    pub fn mark_done(&mut self) {
        self.done = true;
    }
    pub fn fix_link_name(&self) -> Assignment {
        let mut ret = self.clone();
        ret.link_name = Self::get_link_name(ret.name.clone());
        ret
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manager {
    pub jobs: Vec<Job>,
    pub assignments: Vec<Assignment>,
    pub save_file: PathBuf,
}

impl Manager {
    pub fn get_save_file() -> PathBuf {
        let home_dir = home::home_dir().unwrap();
        PathBuf::from(home_dir.join(".calendarthing/manager.json"))
    }
    pub fn empty() -> Self {
        Self {
            jobs: Vec::new(),
            assignments: Vec::new(),
            save_file: PathBuf::from(Self::get_save_file()),
        }
    }

    fn try_from_save_file() -> Result<Self, std::io::Error> {
        let save_file = Self::get_save_file();
        let file = std::fs::read_to_string(save_file)?;
        Ok(serde_json::from_str(&file)?)
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let save_file = Self::get_save_file();
        let json = serde_json::to_string(self)?;
        std::fs::create_dir_all(save_file.parent().unwrap())?;
        std::fs::write(save_file, json)
    }

    fn new_to_file() -> Result<Self, std::io::Error> {
        let manager = Self::empty();
        manager.save()?;
        Ok(manager)
    }

    pub fn from_save_file() -> Self {
        let save_file = Self::get_save_file();
        if !save_file.exists() {
            println!("Creating new save file");
            return Self::new_to_file().unwrap();
        }
        let mut ret = match Self::try_from_save_file() {
            Ok(manager) => manager,
            Err(_) => Self::new_to_file().unwrap(),
        };
        ret.assignments.sort_by(|a, b| {
            if a.due < b.due {
                Ordering::Less
            } else if a.due > b.due {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        ret.assignments = ret
            .assignments
            .iter()
            .map(|a| a.fix_link_name())
            .filter(|a| !a.past_due())
            .collect();
        // _ = ret.save();
        ret
    }

    pub fn add_job(&mut self, job: Job) {
        for j in &mut self.jobs {
            if j.name == job.name {
                j.path = job.path;
                j.sync_to_google = job.sync_to_google;
                j.sync_interval = job.sync_interval;
                j.next_sync = j.last_sync + j.sync_interval;
                return;
            }
        }
        self.jobs.push(job);
    }

    pub fn add_assignment(&mut self, assignment: Assignment) {
        if self.assignments.contains(&assignment) {
            self.assignments.retain(|a| a != &assignment);
        }
        self.assignments.push(assignment);
    }

    pub fn remove_job(&mut self, name: &str) {
        self.jobs.retain(|job| job.name != name);
    }

    pub fn clear_past_due(&mut self) {
        let start_len = self.assignments.len();
        self.assignments.retain(|assignment| !assignment.past_due());
        if start_len != self.assignments.len() {
            println!(
                "Cleared {} past due assignments",
                start_len - self.assignments.len()
            );
        }
    }

    fn get_changed_assignments(&self, assignments: Vec<Assignment>) -> Vec<Assignment> {
        let mut changed: Vec<Assignment> = Vec::new();
        for assignment in &assignments {
            let mut found = false;
            for a in &self.assignments {
                if *a == *assignment {
                    if a.due != assignment.due {
                        changed.push(assignment.clone());
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                changed.push(assignment.clone());
            }
        }
        changed
    }

    fn get_job(&self, name: String) -> Option<&Job> {
        self.jobs.iter().find(|job| job.name == name)
    }

    pub fn should_sync(&self) -> bool {
        for job in &self.jobs {
            if job.sync_due() {
                return true;
            }
        }
        false
    }

    async fn sync_to_google(&mut self, mut assignments: Vec<Assignment>) {
        for assignment in &mut assignments {
            if let Some(job_name) = &assignment.job_name {
                if let Some(job) = self.get_job(job_name.clone()) {
                    if job.sync_to_google {
                        println!("Syncing {} to google", assignment.name);
                        let actual_assignment = self
                            .assignments
                            .iter_mut()
                            .find(|v| v.name == assignment.name)
                            .unwrap_or(assignment);

                        if actual_assignment.synced != None {
                            continue;
                        }
                        let dir = std::env::current_exe()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join("google_sync");
                        println!("Dir: {:?}", dir);
                        let output = Command::new("node")
                            .arg("google.js")
                            .arg(serde_json::to_string(assignment).unwrap())
                            .current_dir(dir)
                            .output()
                            .await
                            .unwrap();
                        println!("Output: {}", String::from_utf8(output.stdout).unwrap());
                        println!("Synced {} to google", assignment.name);
                    }
                }
            }
        }
    }

    fn assignment_match_link_name2(name1: &String, name2: &String, i: usize) -> bool {
        if i >= name1.len() {
            return true;
        }
        let left_char = name1.as_str().chars().nth(i).unwrap();
        let right_char = name2.as_str().chars().nth(i).unwrap();
        if left_char != right_char
            && left_char != '.'
            && left_char != ' '
            && left_char != '/'
            && right_char != '-'
        {
            return false;
        }
        Self::assignment_match_link_name2(name1, name2, i + 1)
    }
    fn assignment_match_link_name(name1: &String, name2: &String) -> bool {
        if name1.len() != name2.len() {
            return false;
        }
        Self::assignment_match_link_name2(name1, name2, 0)
    }
    pub fn get_assignment_from_link_name(&self, name: String) -> Option<&Assignment> {
        for assignment in &self.assignments {
            if Self::assignment_match_link_name(&assignment.name, &name) {
                return Some(assignment);
            }
        }
        None
    }

    pub fn mark_done(&mut self, link_name: String) {
        if let Some(assignment) = self.get_assignment_from_link_name(link_name) {
            let mut assignment = assignment.clone();
            assignment.mark_done();
            self.add_assignment(assignment);
        }
    }

    pub async fn run_jobs(&mut self) {
        let mut assignments: Vec<Assignment> = Vec::new();
        for job in &mut self.jobs {
            if job.sync_due() {
                println!("Syncing {}", job.name);
                let new_assignments = job.sync().await.unwrap();
                println!("Got {} assignments", new_assignments.len());
                assignments.extend(new_assignments);
            }
        }
        let start_len = self.assignments.len();
        let mut changed = self.get_changed_assignments(assignments);
        for assignment in &changed {
            self.add_assignment(assignment.clone());
        }
        for assignment in &mut self.assignments {
            let needs_sync = !assignment.synced.unwrap_or(false);
            assignment.synced = Some(true);
            if needs_sync || assignment.done {
                changed.push(assignment.clone());
            }
        }
        if start_len != self.assignments.len() {
            println!("Added {} assignments", self.assignments.len() - start_len);
        }
        self.sync_to_google(changed).await;
        self.save().unwrap();
    }
}
