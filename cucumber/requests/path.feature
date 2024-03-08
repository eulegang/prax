Feature: Setting paths
    Scenario: Set a path
        Given the path is /
        When filtered req(set(path, "/search"))
        Then path is /search

    Scenario: Set a path with a query present
        Given a query q is attack%20proxy
        And the path is /seach
        When filtered req(set(path, "/foobar"))
        Then path is /foobar
        Then query q is attack%20proxy

