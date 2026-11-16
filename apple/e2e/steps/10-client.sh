# The client against the server (swift test LiveTests).
cd apple && COOEE_PAIR_LINK="$(cat "$E2E/pair")" swift test --filter LiveTests
