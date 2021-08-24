mod customfields;

extern crate goji;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use goji::{Credentials, Jira, SearchOptionsBuilder, Issue};
use std::{env, io};
use std::fs::{File, create_dir};
use std::path::{Path, PathBuf};

#[macro_use]
extern crate prettytable;

use prettytable::{Table};
use std::io::{Read, stdin, Write};
use crate::customfields::Flag;
use dirs::config_dir;


#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    jira_host: Option<String>,
    jira_user: Option<String>,
    jira_pass: Option<String>,
    fields: Vec<String>,
    board: Option<String>,
    project: Option<String>,
    components: Vec<String>,
    status_tables: Vec<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            jira_host: None,
            jira_user: None,
            jira_pass: None,
            fields: vec!["summary".to_string(), "status".to_string(), "components".to_string(), "issuetype".to_string()],
            board: None,
            project: None,
            components: vec!["*".to_string()],
            status_tables: vec![vec!["Done".to_string()], vec!["Review".to_string(), "In progress".to_string(), "Ready".to_string(), "To do".to_string()]],
        }
    }
}

fn get_conf_path() -> PathBuf {
    const CONFIG_FILENAME: &str = "jira.toml";
    let mut conf_path = dirs::config_dir().or(Some(PathBuf::from("."))).unwrap();
    conf_path.push(CONFIG_FILENAME);
    conf_path
}
fn main() {
    let mut used_cache = false;

    let conf = get_conf();
    let conf_clone = conf.clone();

    if let (Some(host), Some(user), Some(pass)) = (
        &conf.jira_host,
        &conf.jira_user,
        &conf.jira_pass
    ) {
        let jira = Jira::new(host, Credentials::Basic(user.to_string(), pass.to_string())).unwrap();

        let sprint = env::var("SPRINT").unwrap_or(env::args().nth(1).expect("No SPRINT given."));
        println!("Sprint: {}", &sprint);
        let board = conf_clone.board.expect("Missing board name in conf.");
        let projname = conf_clone.project.expect("Missing project name in conf.");

        let filename = Path::new("cache").join(&sprint);
        let query = format!("issueFunction not in removedAfterSprintStart(\"{board}\", \"{sprint}\") AND sprint = \"{sprint}\" and Project = \"{proj}\" ORDER BY status", sprint = &sprint, board = board, proj = projname);

        //println!("Query: {}", &query);
        let search = match File::open(&filename) {
            Ok(cache) => {
                used_cache = true;
                serde_cbor::from_reader(cache).unwrap()
            }
            Err(_) => {
                let mut optsb: SearchOptionsBuilder = SearchOptionsBuilder::new();
                optsb.max_results(1000);
                optsb.fields(conf_clone.fields.clone());
                let search = jira.search().list(query, &optsb.build()).unwrap().issues;

                create_dir("cache").unwrap_or(());
                let cachefile = File::create(&filename).expect("Can't create cache file in current dir");
                match serde_cbor::to_writer(&cachefile, &search) {
                    Err(e) => {
                        eprintln!("Error writing to cache file. {}", e.to_string())
                    }
                    Ok(_) => {}
                };
                search
            }
        };
        let conf_clone = conf.clone();
        for component in conf_clone.components.iter() {
            let search = search.clone();
            let by_comp: Vec<&Issue> = if component == "*" {
                search.iter().filter(|issue| issue.components().iter().all(|comp| !conf_clone.components.contains(&comp.name))).collect()
            } else {
                search.iter().filter(|issue| issue.components().iter().any(|comp| comp.name.eq(component))).collect()
            };
            for statuses in conf_clone.status_tables.iter() {
                let by_status: Vec<&&Issue> = by_comp.iter().filter(|&&issue| match issue.status() {

                    Some(status) => {
                        statuses.contains(&"*".to_string()) || statuses.contains(&status.name)
                    }
                    None => { false }
                }).collect();
                let component = if component == "*" { "Others" } else { component };
                println!("******* {} in {} : {} *******", component, statuses.join("/"), by_status.len());
                print_issues(&by_status);
                println!();
            }
            println!();
            println!();
        }
        if used_cache {
            println!("NB! output was printed from cache - making sure it's fresh enough is the user's responsibility. (Check/clear `cache` subfolder.)");
        }
    }
}

fn get_conf() -> Config {
    let mut save_pass = false;
    let mut dirty_conf = false;
    let mut buf = String::new();
    let mut conf_path = get_conf_path();
    println!("{:?}", conf_path);
    let mut conf: Config = File::open(conf_path).map(|mut f| if let Ok(_) = f.read_to_string(&mut buf) {
        toml::from_str(buf.as_str()).unwrap_or_default()
    } else {
        Config::default()
    }).unwrap_or_default();


    conf.jira_host = conf.jira_host.or(env::var("JIRA_HOST").ok());
    conf.jira_user = conf.jira_user.or(env::var("JIRA_USER").ok());
    conf.jira_pass = conf.jira_pass.or(env::var("JIRA_PASS").ok());

    if conf.jira_host.is_none() {
        println!("Jira URL: ");
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        if let Ok(_) = stdin().read_line(&mut buf) {
            conf.jira_host = Some(buf.trim().to_string());
            dirty_conf = true;
        }
    }
    if conf.jira_user.is_none() {
        print!("Jira username: ");
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        if let Ok(_) = stdin().read_line(&mut buf) {
            conf.jira_user = Some(buf.trim().to_string());
            dirty_conf = true;
        }
    }
    if conf.jira_pass.is_none() {
        print!("Jira password: ");
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        if let Ok(_) = stdin().read_line(&mut buf) {
            conf.jira_pass = Some(buf.trim().to_string());
        }
        print!("Save password to file? [y/n] ");
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        if let Ok(_) = stdin().read_line(&mut buf) {
            save_pass = buf.trim().eq_ignore_ascii_case("y");
            dirty_conf = true;
        }
    }
    if dirty_conf {
        let mut outconf = conf.clone();
        if !save_pass {
            outconf.jira_pass = None
        }
        if let Ok(data) = toml::to_string(&outconf) {
            if let Ok(mut file) = File::create(get_conf_path()) {
                match file.write(data.as_bytes()) {
                    Err(e) => {
                        eprintln!("Warning, failed to write config to file. {}", e.to_string());
                    }
                    _ => {}
                };
                file.sync_all().unwrap_or_default();
            }
        }
    }
    conf
}


fn print_issues(issues: &Vec<&&Issue>) {
    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

    for &&issue in issues {
        //println!("{}", issue.fields.keys().map(|f|f.as_str().to_string()).collect::<Vec<String>>().join(", "));
        let flag: Flag = Flag::from(&issue.fields);
        table.add_row(row![format!("{}{}", flag, issue.status().unwrap().name), issue.key, issue.summary().unwrap_or("-".to_owned()), issue.issue_type().unwrap().name ]);
    }
    table.printstd();
}

#[allow(dead_code)]
fn get_all_components(jira: &Jira, project: &str) {
    let comps = jira.project().components(project).expect("Failed to get components");
    for x in comps.iter().map(|c| format!("{}:{}", c.name, c.id)) {
        println!("{}", x);
    }
}
