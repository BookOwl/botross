#[macro_use]
extern crate serenity;
extern crate typemap;

#[macro_use]
extern crate log;
extern crate env_logger;

use serenity::client::{Client, EventHandler};
use serenity::framework::standard::*;
use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::builder::{CreateMessage, CreateEmbed};
use serenity::model::channel::MessageType;
use typemap::Key;

use std::env;
use std::collections::HashMap;

struct Config;
impl Key for Config {
    type Value = ConfigData;
}

struct ConfigData {
    delete_pin_confs: bool,
}
impl Default for ConfigData {
    fn default() -> Self {
        ConfigData {
            delete_pin_confs: false,
        }
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
        ctx.set_game_name("Prefix: \\");
        info!("Rossbot is go!");
    }
    fn resume(&self, _: Context, resume: ResumedEvent) {
        debug!("Resumed; trace: {:?}", resume.trace);
    }
}

pub fn main() {
    env_logger::init().expect("Unable to init env_logger");

    // Login with a bot token from the environment
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
        .expect("Error creating client");
    
    {
        let mut data = client.data.lock();
        data.insert::<Config>(ConfigData::default());
        data.insert::<CommandCounter>(HashMap::new());
    }

    client.with_framework(StandardFramework::new()
                        .configure(|c| c
                              .allow_whitespace(true)
                              .on_mention(true)
                              .prefix("\\"))
                        .before(|ctx, msg, command_name| {
                            info!("Got command '{}' by user '{}'", command_name, msg.author.name);

                            let mut data = ctx.data.lock();
                            let counter = data.get_mut::<CommandCounter>().expect("Expected CommandCounter in ShareMap.");
                            let entry = counter.entry(command_name.to_string()).or_insert(0);
                            *entry += 1;
                            true
                        })
                        .after(|_, _, command_name, error| {
                            match error {
                                Ok(()) => info!("Processed command '{}'", command_name),
                                Err(why) => info!("Command '{}' returned error {:?}", command_name, why),
                            }
                        })
                        .unrecognised_command(|_, msg, cmd| {
                            info!("Unknown command {:?}", cmd);
                            if let Err(e) = msg.channel_id.say(&format!("Unknown command")) {
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
                                        let config = data.get::<Config>().expect("Expected Config in ShareMap.");
                                        config.delete_pin_confs
                                    };
                                    if delete {
                                        info!("Deleting pin conf message");
                                        message.delete();
                                    } else {
                                        info!("Not deleting pin conf message");
                                    }
                                },
                                _ => {},
                            }
                        })
                        .command("ping", |c| c.cmd(ping))
                        .command("launch_nukes", |c| c.check(admin_check).cmd(launch_the_nukes))
                        .command("foo", |c| c.check(owner_check).cmd(foo))
                        .command("delete_pin_confs", |c| c.check(admin_check).cmd(delete_pin_confs))

    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

// A function which acts as a "check", to determine whether to call a command.
//
// In this case, this command checks to ensure you are the owner of the message
// in order for the command to be executed. If the check fails, the command is
// not called.
fn owner_check(_: &mut Context, msg: &Message, _: &mut Args, _: &CommandOptions) -> bool {
    msg.author.id == 270631094657744896
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
                                                    .content("Nukes launched")
                                                    .embed(|e| e.image(r"https://media.giphy.com/media/HhTXt43pk1I1W/giphy.gif"))) {
        error!("Error sending message: {:?}", why);
    }
});

command!(foo(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say("Bar") {
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
        if let Err(why) = msg.channel_id.say(&format!("Ping conf delete status: {}", d)) {
            error!("Error sending message: {:?}", why);
        }
    }
});
