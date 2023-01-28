#[macro_use]
extern crate lazy_static;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

#[macro_use]
extern crate dotenv;

mod gpt3;

use serenity::{
    async_trait,
    builder::CreateApplicationCommands,
    http::CacheHttp,
    model::{
        gateway::Ready,
        id::{ChannelId, GuildId},
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteraction,
                ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
            },
            Interaction, InteractionResponseType,
        },
    },
    prelude::*,
};

use std::env;

fn register_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    commands
        .create_application_command(|command| command.name("ping").description("A ping command"))
        .create_application_command(|command| command.name("reading").description("Tarot Reading"))
}

fn get_string_option(command: &ApplicationCommandInteraction, index: usize) -> Option<String> {
    return command.data.options.get(index).and_then(|option| {
        if let Some(ApplicationCommandInteractionDataOptionValue::String(string)) = &option.resolved
        {
            return Some(string.clone());
        }
        None
    });
}

fn get_user_option(
    command: &ApplicationCommandInteraction,
    index: usize,
) -> Option<serenity::model::user::User> {
    return command.data.options.get(index).and_then(|option| {
        if let Some(ApplicationCommandInteractionDataOptionValue::User(u, _)) = &option.resolved {
            return Some(u.clone());
        }
        None
    });
}

fn get_integer_option(command: &ApplicationCommandInteraction, index: usize) -> Option<i64> {
    return command.data.options.get(index).and_then(|option| {
        if let Some(ApplicationCommandInteractionDataOptionValue::Integer(i)) = &option.resolved {
            return Some(i.clone());
        }
        None
    });
}

fn get_caller_user_id(
    command: &ApplicationCommandInteraction,
) -> Option<serenity::model::id::UserId> {
    let user_id = match command.member.as_ref().map(|m| m.user.id) {
        Some(ref id) => id.clone(),
        None => {
            return None;
        }
    };
    return Some(user_id);
}

struct TarotBotHandler {
    pub gpt3: gpt3::Client,
}

impl TarotBotHandler {
    pub fn new(client: gpt3::Client) -> Self {
        TarotBotHandler { gpt3: client }
    }

    async fn ping(&self, ctx: Context, command: ApplicationCommandInteraction) {
        info!("calling ping!");
        self.send_msg(&ctx, &command, "pong".to_string()).await;
    }

    async fn reading(&self, ctx: Context, command: ApplicationCommandInteraction) {
        lazy_static! {
            static ref tarot_req: gpt3::CompletionRequest = gpt3::CompletionRequest {
                model: "text-davinci-003".to_string(),
                prompt: "Give me a Tarot Reading".to_string(),
                max_tokens: 512,
            };
        }
        info!("calling reading!");

        let _res = self
            .send_msg(
                &ctx,
                &command,
                "Let's see what the spirits wish...".to_string(),
            )
            .await;

        let client = &self.gpt3;

        let res = client.completion(&tarot_req).await;
        match res {
            Ok(res) => {
                let text = &res.choices[0].text;
                let _res = self
                    .say_to(&ctx, command.channel_id, format!("{}", text))
                    .await;
            }
            Err(err) => {
                self.send_error(ctx, command, format!("I got an error: {}", err))
                    .await;
            }
        }
    }

    async fn say_to(&self, ctx: &Context, channel: ChannelId, msg: String) {
        let res = channel.say(&ctx.http, msg).await;
        if let Err(e) = res {
            error!("{}", e);
        }
    }

    async fn send_msg(&self, ctx: &Context, command: &ApplicationCommandInteraction, msg: String) {
        let res = command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content(msg))
            })
            .await;

        if let Err(e) = res {
            error!("send message err: {}\n {:?}", e, command);
        }
    }

    async fn send_interaction_error(
        &self,
        ctx: Context,
        command: ApplicationCommandInteraction,
        e: String,
    ) {
        error!("send_interaction_err: {}", e);
        let res = command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("Oops. Something went wrong.")
                    })
            })
            .await;

        if let Err(e) = res {
            error!("send_interaction_err_failed: {}", e);
        }
    }

    async fn send_error(&self, ctx: Context, command: ApplicationCommandInteraction, e: String) {
        error!("send_error: {}", e);
        self.say_to(
            &ctx,
            command.channel_id,
            "Oops. Something went wrong.".to_string(),
        )
        .await;
    }
}

#[async_trait]
impl EventHandler for TarotBotHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "ping" => {
                    self.ping(ctx, command).await;
                }
                "reading" => {
                    self.reading(ctx, command).await;
                }

                _ => error!("err unimplemented"),
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("[{}]{} is connected!", ready.user.id, ready.user.name);

        let commands = ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
            register_commands(commands)
        })
        .await
        .expect("couldn't create application commands");

        info!("The following commands are available: {:#?}", commands);

        let _guild_command = GuildId(494671450985201665)
            .create_application_command(&ctx.http, |command| {
                command.name("ping").description("A test ping command")
            })
            .await
            .expect("failed to create guild command");

        let _guild_command = GuildId(494671450985201665)
            .create_application_command(&ctx.http, |command| {
                command.name("reading").description("Tarot Reading")
            })
            .await
            .expect("failed to create guild command");
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    dotenv::dotenv().expect("Failed to read .env file");
    pretty_env_logger::init();
    trace!("trace enabled");
    debug!("debug enabled");
    info!("info enabled");
    warn!("warn enabled");
    error!("error enabled");
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let gpt3_token = env::var("OPENAI_KEY").expect("Expected a OPENAI_KEY in the environment");
    let gpt3_client = gpt3::Client::new(gpt3_token);

    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    // Build our client.
    let mut client = Client::builder(token)
        .event_handler(TarotBotHandler::new(gpt3_client))
        .application_id(application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn write_firebase() {}
}
