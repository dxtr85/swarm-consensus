# An implementation of Swarm Consensus
# Links
You can read more about Swarm Consensus in [this document](https://drive.google.com/file/d/1Wc8lbKfHnpwTncVnavTYbvsK2wNGpetg/view?usp=sharing) ( [also on Dropbox](https://www.dropbox.com/scl/fi/1pq3vcjjvf8yfblo9yt7f/schmebulock.pdf?rlkey=r79bfau1mzgt4ozi99irvtgig&st=d7fv8e4s&dl=0) ).

# TODOs
We need to support broadcasting scenarios above the basic one, which is
to send Data from Origin to all Gnomes present in the Swarm during Broadcast start.
Some additional scenarios are:
- Provide a way for a Gnome to Synchronize Swarm state, so that he is aware of
  active broadcasts and multicasts (direct to Neighbor or multiple Neighbors)
- Allow a Gnome to subscribe to an existing broadcast (direct to Neighbor)
- Allow a Gnome to unsubscribe from broadcast (direct to Neighbor)
- Allow a Gnome to take-over a broadcast once inactivity period has passed
- Allow a Gnome to end a Broadcast once inactivity period has passed
- Allow a Gnome to send a message uplink to Broadcast origin in order to
  notify subscribers (can be used for a service broadcast that provides information
  about other Gnomes open for joining defined Swarms without changing Swarm's state)
  (direct to Neighbor and recursive up to origin)
- In order to implement load balancing we also need a way for source Gnomes to notify 
  their subscribers that they should switch given broadcast to a different source.
  Upon such notification receiving Gnome should respond with Unsubscribe message.
  (direct to Neighbor)


Once Swarm state sync is in place we can think of following:
Introduction of Capabilities, together with Groups and Actions. Caps should be
used for verifying that requesting Gnome is allowed to perform requested Action.
Capabilities, Groups and Actions should be defined by application and Actions should
be able to extend or shrink existing sets of any and all of those, making the system
dynamic.
Groups should also be used to define target recipients of Multicasts, but that requires
implementation of encrypted casting and is lower priority.
Maybe we should have a separate CastingGroup and Group for Capabilities.

