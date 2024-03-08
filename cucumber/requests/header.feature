Feature: Setting paths
    Scenario: Setting Header
        Given the method is GET
        And a header Authentication is Bearer token
        When filtered req(set(header("Authentication"), "Bearer foobarxyz"))
        Then method is GET
        And header Authentication is Bearer foobarxyz

