* enfire task .run should be .eval and return whether or not the client is abusive
* allow configurable "abuse actions" when the abuse threshold is reached
* allow configurable storage backend
* maybe allow dynamic configuration from a http source
* some builtin abuse actions could be
    * add to iptables/nftables (find which is available in PATH)
    * notify a webhook/http endpoint
    * run a shell command
    * run a (provided) lua script

