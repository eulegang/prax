Feature: Setting queries
    Scenario: empty queries
        Given the method is GET
        When filtered req(set(query("q"), "hello"))
        Then query q is hello

    Scenario: adds additional query
        Given the method is GET
        And a query q is world
        When filtered req(set(query("q"), "hello"))
        Then query q is hello
        Then query q is world

