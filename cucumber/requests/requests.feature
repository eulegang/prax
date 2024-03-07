Feature: Requests
    Scenario: Method set
        Given a PUT request
        When filtered req(set(method, "POST"))
        Then method is POST

    Scenario: Set a path
        Given a GET request
        When filtered req(set(path, "/search"))
        Then path is /search

    Scenario: Set a path with query
        Given a GET request
        And a query q is hello
        When filtered req(set(path, "/search"))
        Then path is /search

    Scenario: Setting Header
        Given a GET request
        And a header Authentication is Bearer token
        When filtered req(set(header("Authentication"), "Bearer foobarxyz"))
        Then method is GET
        And header Authentication is Bearer foobarxyz

