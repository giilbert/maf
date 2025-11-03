!["mutation authority framework" banner](assets/banner.png)

<hr>

A toolkit for making realtime apps.

## Motivation

__Making realtime apps is a PAIN IN THE A*$ ☹️☹️☹️.__

Suppose you want to make a Kahoot-like realtime quiz app. Broken down, you will need to:
- Create a server that can handle multiple clients connecting at the same time
- Handle user authentication and authorization (e.g. who is the host, who are the players, etc.)
- Send realtime updates to players (e.g. when a player answers a question, the host needs to see the updated scores, etc.)
- Handle user input (e.g. when a player answers a question, the host needs to see the updated scores, etc.)
- Keep track of the game's state (e.g. question number, player scores, whether the game is waiting for players/finished, etc.)

__Option 1__: Use a server-side library like Socket.IO to handle the realtime communication, and then add a bunch of code to handle the game logic.

- A s___ ton of work
- A s___ ton of bugs
- Feels like a mess working with ✨ state ✨ on the server and client

__Option 2__: Use a client-side library like Firebase or Liveblocks to handle the realtime communication, with code running on the client being the authoritative source of truth.

- Points 1 & 2 from above
- But now you have to deal with the fact that the __client is not a trusted source of truth__. This may not be a problem for a quiz app, but if you want to make a game where integrity is important, you will need a server to handle the game logic.

## Mutation "Authority" Framework

__The core problem with realtime apps is managing ✨ STATE ✨.__

Existing tools make it feel unnatural to work with state. Why can't we have ...

```rust
fn handle_request(state: &mut State, request: Request) -> Response {
    // ...
}
```

... for realtime apps?

MAF is a server framework aiming to abstract away the complexity of managing state in realtime apps, so you can focus on actually building your app instead of confusing yourself with boilerplate.

MAF also provides a set of tools to help you easily deploy your app. So, next time you want to make a realtime app (say... a hackathon), you can just use MAF and get it online in a few minutes.

### Features 🌈🦄 
- __Realtime State Abstractions__
- __WASM Deployments__
- __Client & Server Libraries__
- TODO TODO TODO

## Getting Started
uhhhhhhhh to be continued