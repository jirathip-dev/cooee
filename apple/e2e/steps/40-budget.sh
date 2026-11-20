# #606: the app's own timings the steps above posted to the server's perf log (tap to painted, event to painted, send
# to echo) fail the run when a p95 is over its budget: tap 100 ms, event and echo 150 ms.
sh -c "$(sed 's/ pair / perf --budget /' "$E2E/pair.again")"
