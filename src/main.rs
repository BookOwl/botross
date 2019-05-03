#[macro_use]
extern crate serenity;
extern crate typemap;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate subprocess;

#[macro_use]
extern crate postgres;

#[macro_use]
extern crate lazy_static;

use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::client::{Client, EventHandler};
use serenity::framework::standard::*;
use serenity::model::channel::MessageType;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::time::Duration;
use subprocess::{Exec, Redirection};
use typemap::Key;
use postgres::TlsMode;
use postgres::tls::native_tls::NativeTls;

/// My Discord ID. Replace this with your user ID
const OWNER_ID: u64 = 270_631_094_657_744_896;

const ABOUT: &str = r#"
BotRoss is a Discord bot created by Matthew Stanley and released under the MIT license.
For a list of commands type `\help`
To see the license type `\license`

Source code for BotRoss can be found at https://github.com/BookOwl/botross/
"#;

const LICENSE: &str = include_str!("../LICENSE");

#[derive(Debug, Clone)]
struct HelpItem {
    usage: &'static str,
    short: &'static str,
    long: &'static str,
}

lazy_static! {
    static ref HELP_TEXTS: HashMap<&'static str, HelpItem> = {
        [
        ("about", HelpItem {
            usage: "\\about",
            short: "",
            long: "Displays information about BotRoss and links to the source code"
        }),
        ("license", HelpItem {
            usage: "\\license",
            short: "",
            long: "Displays the license for BotRoss (the MIT license)"
        }),
        ("ping", HelpItem {
            usage: "\\ping",
            short: "(Test command)",
            long: "Sends \"Pong!\" (for testing)",
        }),
        ("V2", HelpItem {
            usage: "\\V2",
            short: "",
            long: "This is what +V2 was for"
        }),
        ("delete_pin_confs", HelpItem {
            usage: "\\delete_pin_confs [yes|no|true|false]",
            short: "Sets BotRoss to automatically delete pin conf messages",
            long: r#"BotRoss can automatically delete pin confirmation messages. 
If called with no arguments displays the current pin deletion status, 
otherwise if called with [yes|no|true|false] sets the pin confirmation setting"#
        }),
        ("py", HelpItem {
            usage: "\\py (code)",
            short: "Runs Python3 code",
            long: r#"Evaluates the Python3 code passed and prints the result.
Can take the (code) argument either in a tripple backtick code block for one or more statements or as the rest of the comment for an expresion."#
        }),
        ("help", HelpItem {
            usage: "\\help [cmd]",
            short: "Displays help",
            long: "Displays a list of commands if run without arguments or help for a specified command."
        }),
        ].iter().cloned().collect::<HashMap<&str,_>>()
    };
}

struct Config;
impl Key for Config {
    type Value = ConfigData;
}

#[derive(Debug)]
struct ConfigData {
    delete_pin_confs: bool,
}
impl Default for ConfigData {
    fn default() -> Self {
        ConfigData {
            delete_pin_confs: true,
        }
    }
}
impl ConfigData {
    fn load_from_db() -> ConfigData {
        let conn = connect_to_db();
        conn.execute("CREATE TABLE IF NOT EXISTS config (
                        id                  SERIAL PRIMARY KEY,
                        delete_pin_confs    BOOL
                     )", &[]).unwrap();
        if let Some(row) = conn.query("SELECT delete_pin_confs FROM config", &[]).unwrap().iter().next() {
            ConfigData {
                delete_pin_confs: row.get(0),
            }
        } else {
            let default_config: ConfigData = Default::default();
            conn.execute("INSERT INTO config (delete_pin_confs) VALUES ($1)",
                 &[&default_config.delete_pin_confs]).unwrap();
            default_config
        }
    }
    fn save_to_db(&self) {
        let conn = connect_to_db(); 
        conn.execute("CREATE TABLE IF NOT EXISTS config (
                        id                  SERIAL PRIMARY KEY,
                        delete_pin_confs    BOOL
                     )", &[]).unwrap();
        conn.execute("DELETE FROM config", &[]).unwrap();
        conn.execute("INSERT INTO config (delete_pin_confs) VALUES ($1)",
                 &[&self.delete_pin_confs]).unwrap();
    }
}

fn connect_to_db() -> postgres::Connection {
    let DB_URL = env::var("DATABASE_URL").unwrap();
        info!("{:?}", DB_URL);
        let negotiator = NativeTls::new().unwrap();
        if let Ok(conn) = postgres::Connection::connect(DB_URL.as_str(), TlsMode::Require(&negotiator)) {
            conn
        } else {
            postgres::Connection::connect(DB_URL.as_str(), TlsMode::None).unwrap()
        }
}

struct CommandCounter;

impl Key for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, r: Ready) {
        info!("{} is connected!", r.user.name);
        ctx.set_game(Game::playing("Prefix: \\"));
        info!("BotRoss is go!");
    }
    fn resume(&self, _: Context, resume: ResumedEvent) {
        debug!("Resumed; trace: {:?}", resume.trace);
    }
}

pub fn main() {
    env_logger::init().expect("Unable to init env_logger");

    // Login with a bot token from the environment
    info!("{:?}", &env::var("DISCORD_TOKEN").expect("token"));
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
        .expect("Error creating client");
    info!("Created client");

    {
        let mut data = client.data.lock();
        data.insert::<Config>(ConfigData::load_from_db());
        data.insert::<CommandCounter>(HashMap::new());
    }
    info!("Created config data");

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.allow_whitespace(true).on_mention(true).prefix("\\"))
            .before(|ctx, msg, command_name| {
                info!(
                    "Got command '{}' by user '{}'",
                    command_name, msg.author.name
                );

                let mut data = ctx.data.lock();
                let counter = data
                    .get_mut::<CommandCounter>()
                    .expect("Expected CommandCounter in ShareMap.");
                let entry = counter.entry(command_name.to_string()).or_insert(0);
                *entry += 1;
                true
            })
            .after(|_, _, command_name, error| match error {
                Ok(()) => info!("Processed command '{}'", command_name),
                Err(why) => info!("Command '{}' returned error {:?}", command_name, why),
            })
            .unrecognised_command(|_, msg, cmd| {
                info!("Unknown command {:?}", cmd);
                if let Err(e) = msg.channel_id.say("Unknown command".to_string()) {
                    error!("Error sending messege: {:?}", e);
                }
            })
            .message_without_command(|ctx, message| {
                info!("Message is not a command '{}'", message.content);
                match message.kind {
                    MessageType::PinsAdd => {
                        info!("Message is a pin notification");
                        let delete = {
                            let data = ctx.data.lock();
                            let config =
                                data.get::<Config>().expect("Expected Config in ShareMap.");
                            config.delete_pin_confs
                        };
                        if delete {
                            info!("Deleting pin conf message");
                            if let Err(e) = message.delete() {
                                error!("Error deleting pin conf message: {:?}", e);
                            };
                        } else {
                            info!("Not deleting pin conf message");
                        }
                    }
                    _ => {}
                }
            })
            .command("ping", |c| c.cmd(ping))
            .command("V2", |c| {
                c.check(admin_check).cmd(launch_the_nukes)
            })
            .command("delete_pin_confs", |c| {
                c.check(admin_check).cmd(delete_pin_confs)
            })
            .command("py", |c| c.check(admin_check).cmd(py))
            .command("about", |c| c.cmd(about))
            .command("license", |c| c.cmd(license))
            .command("help", |c| c.cmd(help)),
    );
    info!("framework created");

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        error!("An error occurred while running the client: {:?}", why);
    }
}

// A function which acts as a "check", to determine whether to call a command.
//
// In this case, this command checks to ensure you are the owner of the message
// in order for the command to be executed. If the check fails, the command is
// not called.
fn owner_check(_: &mut Context, msg: &Message, _: &mut Args, _: &CommandOptions) -> bool {
    msg.author.id == OWNER_ID
}

// A function which acts as a "check", to determine whether to call a command.
//
// This check analyses whether a guild member permissions has
// administrator-permissions.
fn admin_check(_: &mut Context, msg: &Message, _: &mut Args, _: &CommandOptions) -> bool {
    if let Some(member) = msg.member() {
        if let Ok(permissions) = member.permissions() {
            return permissions.administrator();
        }
    }

    false
}

command!(ping(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say("Pong!") {
        error!("Error sending message: {:?}", why);
    }
});

command!(launch_the_nukes(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.send_message(|m| m
                                                    .content("This is what +V2 was for")
                                                    .embed(|e| e.image(r"https://media.giphy.com/media/HhTXt43pk1I1W/giphy.gif"))) {
        error!("Error sending message: {:?}", why);
    }
});

command!(delete_pin_confs(ctx, msg, args) {
    let yes = ["y", "yes", "true", "1"];
    let arg = if args.is_empty() { String::from("status") } else { args.single::<String>()?.to_lowercase() };
    let d = yes.contains(&arg.as_str());
    let status = {
        let data = ctx.data.lock();
        let config = data.get::<Config>().expect("Expected Config in ShareMap.");
        config.delete_pin_confs
    };
    if arg == "status" {
        if let Err(why) = msg.channel_id.say(&format!("Ping conf delete status: {}", status)) {
            error!("Error sending message: {:?}", why);
        }
    } else {
        let mut data = ctx.data.lock();
        let mut config = data.get_mut::<Config>().expect("Expected Config in ShareMap.");
        config.delete_pin_confs = d;
        config.save_to_db();
        if let Err(why) = msg.channel_id.say(&format!("Ping conf delete status: {}", d)) {
            error!("Error sending message: {:?}", why);
        }
    }
});

#[derive(Debug, PartialEq)]
enum PyMode {
    Expression,
    Program,
}
command!(py(_ctx, msg, args) {
    let code = args.full();
    let mode = if code.starts_with("```") {
        PyMode::Program
    } else {
        PyMode::Expression
    };
    let code = code.trim_start_matches("`").trim_start_matches("python\n").trim_start_matches("py\n").trim_end_matches("`");
    let original_code = code;
    let code = match mode {
        PyMode::Expression => format!("print({})", code),
        PyMode::Program => code.to_string(),
    };
    info!("mode: {:?} code: {:?}", mode, code);
    let mut f = File::create("temp.py")?;
    write!(&mut f, "{}\n", code)?;
    f.sync_data()?;
    let mut p = (if cfg!(windows) {Exec::cmd("py").arg("-3")} else {Exec::cmd("python3")}).arg("temp.py").stdout(Redirection::Pipe).stderr(Redirection::Merge).popen()?;
    if let Some(status) = p.wait_timeout(Duration::new(5, 0))? {
        info!("python process finished as {:?}", status);
        let mut b = String::new();
        p.stdout.as_mut().unwrap().read_to_string(&mut b)?;
        let res = match mode {
            PyMode::Expression => format!("```py\n>>> {}\n{}\n```" , original_code, b),
            PyMode::Program => format!("Result:\n```\n{}\n```", b),
        };
        if let Err(why) = msg.channel_id.say(&res) {
            error!("Error sending message: {:?}", why);
        }
    } else {
        p.kill()?;
        p.wait()?;
        info!("python process killed");
        if let Err(why) = msg.channel_id.say("Process timed out. :(") {
            error!("Error sending message: {:?}", why);
        }
    }
});

command!(about(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say(ABOUT) {
        error!("Error sending message: {:?}", why);
    }
});

command!(license(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say(&format!("License for BotRoss:```\n{}\n```", LICENSE)) {
        error!("Error sending message: {:?}", why);
    }
});

command!(help(_ctx, msg, args) {
    if args.is_empty() {
        let mut help_txt = "Commands:\n```\n".to_owned();
        for (_cmd_name, help_info) in HELP_TEXTS.iter() {
            if help_info.short.len() > 0 { 
                help_txt.push_str(&format!("{}: {}\n\n", help_info.usage, help_info.short));
            } else {
                help_txt.push_str(help_info.usage);
                help_txt.push_str("\n\n");
            }
        }
        help_txt.push_str("```");
        info!("Help txt: {:?}", help_txt);
        if let Err(why) = msg.channel_id.say(&help_txt) {
            error!("Error sending message: {:?}", why);
        }
    } else {
        let arg: String = args.single()?;
        let response = if let Some(help_info) = HELP_TEXTS.get::<&str>(&&arg.as_str()) { // Hideous but works somehow
            format!("{}\nUsage: `{}`\n\n{}", arg, help_info.usage, help_info.long)
        } else {
            format!("`\\{}` is not a known command", arg)
        };
        if let Err(why) = msg.channel_id.say(&response) {
            error!("Error sending message: {:?}", why);
        }
    };

});
