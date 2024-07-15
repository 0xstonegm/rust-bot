## Todos
- [ ] Add run configuration for running multiple strategies at the same time.
- [ ] Implement more strategies to be run in parallel.
- [ ] Big Dick Bot!
- [ ] Find some way of restarting the bot once it hits an error state. Currently it can fail but the results get handled and then it just remains going forever in the loop although the websockets are disconnected etc. Better to either fail and restart or ensure that all possible fail states are properly handled. 
- [ ] Install undotree
- [ ] Fix bug where the same strategy can have multiple trades active at once. 
- [ ] Improve speed of indicator population when multiple strategies are run in parallell. This can be accomplished by parallellizing the get indicator population inside multiple_strategies.js using the rayon crate.
- [ ] Remove netversion
