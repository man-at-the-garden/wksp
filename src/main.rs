use std::env;
use std::fs::{self, remove_file};
use std::path::{Path, PathBuf};
use std::os::unix::fs::symlink;
use std::io::{self, Error};
use std::collections::HashMap;
use std::collections::HashSet;


fn main() {
    let home_dir = dirs::home_dir().expect("Failed to get home directory").display().to_string();

    let mut files = Vec::new();
    files.push(FilePath { target_path: format!("{}", home_dir), file_name: String::from(".gitconfig") });
    files.push(FilePath { target_path: format!("{}/.ssh", home_dir), file_name: String::from("id_rsa") });
    files.push(FilePath { target_path: format!("{}/.ssh", home_dir), file_name: String::from("id_rsa.pub") });
    files.push(FilePath { target_path: format!("{}", home_dir), file_name: String::from(".gnupg") });
    files.push(FilePath { target_path: format!("{}", home_dir), file_name: String::from(".password-store") });
    files.push(FilePath { target_path: format!("{}/.config", home_dir), file_name: String::from("snipets") });

    let args: Vec<String> = env::args().collect();
    let argument = &args[1];
    let snapshot = match Snapshot::now() {
        Ok(v) => v,
        Error => panic!("Snapshot creation failed"),
    };

    let (change, workspace) = match argument.as_str() {
        "togglew" => (true, snapshot.nextw()),
        "togglee" => (true, snapshot.nexte()),
        "show" => (false, snapshot.current()),
        _ => (false, snapshot.current()),
    };

    println!("[{}] - [{}]", workspace.main_dir, workspace.env_dir);

    if !change {
        return;
    }

    for file in files {
        workspace.update_link(&file);
    }

    workspace.write();
}

#[derive(Debug)]
struct Snapshot {
    current_workspace: usize,
    current_environment: usize,
    workspace_list: Vec<String>,
    environment_map: HashMap<String, Vec<String>>
}


impl Snapshot {
    fn current(&self) -> WorkspaceConfig {
        let main_dir = self.workspace_list.get(self.current_workspace)
            .expect("Could not get workspace from snapshot list").to_string();

        let env_dir = self.environment_map.get(&main_dir)
            .expect("Could not get workspace from snapshot map").get(self.current_environment)
            .expect("Could not get environment from snapshot map list").to_string();

        WorkspaceConfig {
            main_dir,
            env_dir
        }
    }

    fn nextw(&self) -> WorkspaceConfig {
        let target_workspace = if self.current_workspace == self.workspace_list.len() - 1 { 0 } else { self.current_workspace + 1 };
        let main_dir = self.workspace_list.get(target_workspace)
            .expect("Could not get workspace from snapshot list").to_string();

        let target_environment = 0;
        let env_dir = self.environment_map.get(&main_dir)
            .expect("Could not get environment list from snapshot map").get(target_environment)
            .expect("Could not get environment from snapshot map list").to_string();

        WorkspaceConfig {
            main_dir,
            env_dir
        }
    }

    fn nexte(&self) -> WorkspaceConfig {
        let target_workspace = self.current_workspace;
        let main_dir = self.workspace_list.get(target_workspace)
            .expect("Could not get workspace from snapshot list").to_string();

        let environment_list = self.environment_map.get(&main_dir)
            .expect("Could not get workspace from snapshot map");

        let target_environment = if self.current_environment == environment_list.len() - 1 { 0 } else { self.current_environment + 1 };
        let env_dir = environment_list.get(target_environment)
            .expect("Could not get environment from snapshot map list").to_string();

        WorkspaceConfig {
            main_dir,
            env_dir
        }
    }

    fn now() -> Result<Self, io::Error> {
        let home_dir = dirs::home_dir().expect("Failed to get home directory");
        let workspaces_path = home_dir.join("workspace");
        let current_path = workspaces_path.join("current");

        let workspace_string = fs::read_to_string(current_path.join("wsp"))?.trim().to_string();
        let environment_string = fs::read_to_string(current_path.join("env"))?.trim().to_string();

        let workspace_list = get_sub_dirs(&workspaces_path)?;
        let mut environment_map: HashMap<String, Vec<String>> = HashMap::new();

        for workspace in &workspace_list {
            let path = workspaces_path.join(workspace);
            let sub_dirs = get_sub_dirs(&path)?;
            environment_map.insert(workspace.clone(), sub_dirs);
        }

        let current_workspace = workspace_list
            .iter()
            .position(|x| x == &workspace_string)
            .expect("Workspace not found");

        let current_environment = environment_map
            .get(&workspace_string)
            .and_then(|env_list| {
                env_list
                    .iter()
                    .position(|x| x == &environment_string)
            })
            .expect("Environment not found");

        Ok(Snapshot {
            current_workspace,
            current_environment,
            workspace_list,
            environment_map,
        })
    }
}

fn get_sub_dirs(path: &PathBuf) -> Result<Vec<String>, io::Error> {
    let exclude_set: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("default");
        set.insert("current");
        set
    };

    let mut entries: Vec<String> = fs::read_dir(path)?
        .filter(|entry| {
            entry.as_ref()
                .map(|entry| entry.metadata().map(|metadata| metadata.is_dir()).unwrap_or(false))
                .unwrap_or(false)
        })
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .filter(|s| !exclude_set.contains(s.as_str()))
        .collect();

    entries.sort();

    Ok(entries)
}

struct FilePath {
    target_path: String,
    file_name: String
}

impl FilePath {
    fn refresh_link_from(&self, new_source_dir: Option<PathBuf>) -> Result<(), Error> {
        let target_link = Path::new(&self.target_path).join(&self.file_name);

        if target_link.is_symlink() {
            remove_file(&target_link)?;
        }

        match new_source_dir {
            Some(dir) => {
                let source_path = dir.join(&self.file_name);
                //println!("{}, {:?}", target_link.display(), source_path);
                symlink(source_path, &target_link)?;
            }
            None => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
struct WorkspaceConfig {
    main_dir: String,
    env_dir: String
}

impl WorkspaceConfig {
    fn update_link(&self, file_path: &FilePath) -> Result<(), Error> {
        let home_dir = dirs::home_dir().expect("Failed to get home directory");
        let workspace_dir = home_dir.join("workspace").join(&self.main_dir);

        if !workspace_dir.is_dir() {
            return Err(Error::new(io::ErrorKind::NotFound, "Workspace directory not found"));
        }

        let environment_dir = workspace_dir.join(&self.env_dir);
        let default_dir = workspace_dir.join("default");

        let source_dir = if environment_dir.is_dir() && environment_dir.join(&file_path.file_name).exists() {
            Some(environment_dir)
        } else if default_dir.is_dir() && default_dir.join(&file_path.file_name).exists() {
            Some(default_dir)
        } else {
            None
        };

        file_path.refresh_link_from(source_dir)
    }

    fn write(&self) -> Result<(), Error> {
        let home_dir = dirs::home_dir().expect("Failed to get home directory");
        let workspaces_path = home_dir.join("workspace");
        let current_path = workspaces_path.join("current");

        fs::write(current_path.join("wsp"), &self.main_dir)?;
        fs::write(current_path.join("env"), &self.env_dir)?;

        Ok(())
    }

}
