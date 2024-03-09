Feature: Setting paths
    Scenario: Setting Header with it being present
        Given the method is GET
        And a header Authentication is Bearer token
        When filtered req(set(header("Authentication"), "Bearer foobarxyz"))
        Then method is GET
        And header Authentication is Bearer foobarxyz

    Scenario: Setting Header without it set
        Given the method is GET
        When filtered req(set(header("Authentication"), "Bearer foobar"))
        Then method is GET
        And header Authentication is Bearer foobar

