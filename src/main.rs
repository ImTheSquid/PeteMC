use std::env;
use lazy_static::lazy_static;
use microkv::MicroKV;
use serenity::async_trait;
use serenity::model::Permissions;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::model::prelude::{Ready, GuildId, interaction::Interaction};
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "status" => {
                    let res = DB.exists("address");

                    if res.is_err() || !res.unwrap() {
                        ":x: Server info is not configured".to_string()
                    } else {
                        let address: String = if let Some(option) = command.data.options.get(0) {
                            option.value.as_ref().unwrap().as_str().unwrap().to_string()
                        } else {
                            DB.get_unwrap("address").unwrap()
                        };

                        let connection = async_minecraft_ping::connect(address.clone()).await.unwrap();

                        match connection.status().await {
                            Ok(res) => if res.status.players.online > 0 {
                                format!(
                                    ":white_check_mark: Server `{}` is online with `{}/{}` players. Currently Online: `{}`", 
                                    address,
                                    res.status.players.online, 
                                    res.status.players.max,
                                    res.status.players.sample.unwrap_or(vec![]).into_iter().map(|player| player.name).collect::<Vec<_>>().join(", ")
                                )
                            } else {
                                format!(
                                    ":white_check_mark: Server `{}` is online with `0/{}` players.", 
                                    address, 
                                    res.status.players.max
                                )
                            },
                            Err(e) => format!(":x: Failed to fetch status: {}", e.to_string())
                        }
                    }
                },
                "set_address" => {
                    let address = &command.data.options.get(0).unwrap().value.as_ref().unwrap().as_str().unwrap().to_string();
                    
                    if let Err(e) = DB.put("address", address) {
                        format!(":x: Failed to set address: {}", e.to_string())
                    } else {
                        format!(":white_check_mark: Address set to `{}`", address)
                    }
                },
                _ => "Invalid command!".to_string()
            };

            if let Err(e) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                println!("Cannot respond to slash command: {}", e);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        guild_id.set_application_commands(ctx, |commands| {
            commands
                .create_application_command(|command| {
                    command
                    .name("status")
                    .description("Gets status of the default server or an alternate if given")
                    .create_option(|option| {
                        option
                            .name("address")
                            .description("The address to check if specified, otherwise the default address will be used")
                            .kind(CommandOptionType::String)
                            .required(false)
                    })
                })
                .create_application_command(|command| {
                    command
                        .name("set_address")
                        .description("Sets the default server address")
                        .create_option(|option| {
                            option
                                .name("address")
                                .description("The address")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                        .default_member_permissions(Permissions::ADMINISTRATOR)
                })
        }).await.unwrap();
    }
}

lazy_static! {
    static ref DB: MicroKV = {
        let db = MicroKV::open_with_base_path("petemc_db", env::current_exe().unwrap().parent().unwrap().to_path_buf());

        db.expect("Failed to create MicroKV!").set_auto_commit(true).with_pwd_clear(env::var("DB_PASSWD").unwrap())
    };
}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("No token provided!");
    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(e) = client.start().await {
        println!("An error occurred while running the client: {:?}", e);
    }
}