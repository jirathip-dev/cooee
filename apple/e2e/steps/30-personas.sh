# #139: the personas the simulator made and edited, read back from the server, and a client round trip; skipped when
# $PERSONA is false (the scope's `persona`).
[ "${PERSONA:-true}" = true ] || exit 0
cd apple && COOEE_PERSONA_LINK="$(pair_link)" COOEE_SIM_LOOKS="dark light" swift test --filter LivePersonaTests
