# verify-229-r11 transcript (recipe .herdsman/verify.md, iOS; DROVER_PORT=41257, DROVER_LANE for HERDSMAN_*)

ios doctor exit=0
(serve.sh launched under setpgrp, pid in .verify/serve.pid)
ios boot exit=0                 (iPhone 17, iOS 27.2)
ios build exit=0                ** TEST BUILD SUCCEEDED **
ios serve ready after 1s exit=0
ios drive dark exit=0           ** TEST EXECUTE SUCCEEDED ** 12 tests, 2 skipped, 0 failures
ios export dark exit=0
ios drive light exit=0          ** TEST EXECUTE SUCCEEDED ** 12 tests, 2 skipped, 0 failures
ios export light exit=0
ios kill exit=0
ios sim delete exit=0
ios curl exit=7 (7: nothing listens)
ios sim listed exit=1 (1: gone)
