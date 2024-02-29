Feature: Requests
    Scenario: Method set
        Given a PUT request
        When filtered target("example.com:3000"):req(set(method, "POST"))
        Then method is POST

    Scenario: Setting Header
        Given a GET request
        And a header Authentication: Bearer token
        When filtered target("example.com:3000"):req(set(header("Authentication"), "Bearer foobarxyz"))
        Then method is GET
        And header Authentication is Bearer foobarxyz

