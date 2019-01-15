# botross
A discord bot I made to help out with my Discord servers and to learn how Discord bots work.

## Running BotRoss
First you need to create a bot account and get its token. See the [Discord Docs](https://discordapp.com/developers/docs/intro) for more info.

Then you need to put your Discord token in either [runbot.bat](runbot.bat) (if you are on Windows) or [the run script](run) (if you are on MacOS/Linux). Then you can run the correct script to build and run BotRoss.

## Running BotRoss on Heroku
BotRoss can be run on Heroku. Just clone this repo, run `heroku create --buildpack emk/rust`, set the `DISCORD_TOKEN` config variable to your discord token and the `RUST_LOG` config variable to `info` (or whatever logging level you want), and then `git push heroku master` to deploy.

## License
Released under the MIT license. See [LICENSE](LICENSE) for details.