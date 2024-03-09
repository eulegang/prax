Feature: Setting headers
    Scenario: Setting Header without it being present
        Given the status is 200
        When filtered resp(set(header("server"), "foobar"))
        Then header server is foobar

    Scenario: Setting Header without it being present
        Given the status is 200
        When filtered resp(set(header("server"), "foobar"))
        Then header server is foobar
