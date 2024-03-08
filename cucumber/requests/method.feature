Feature: Setting methods
    Scenario: setting put
        Given the method is PUT
        When filtered req(set(method, "PUT"))
        Then method is PUT

    Scenario: setting post
        Given the method is PUT
        When filtered req(set(method, "POST"))
        Then method is POST

    Scenario: setting get
        Given the method is GET
        When filtered req(set(method, "GET"))
        Then method is GET

    Scenario: setting trace
        Given the method is TRACE
        When filtered req(set(method, "TRACE"))
        Then method is TRACE

    Scenario: setting head
        Given the method is HEAD
        When filtered req(set(method, "HEAD"))
        Then method is HEAD

    Scenario: setting patch
        Given the method is HEAD
        When filtered req(set(method, "PATCH"))
        Then method is PATCH

    Scenario: setting delete
        Given the method is DELETE
        When filtered req(set(method, "DELETE"))
        Then method is DELETE

    Scenario: setting lock (non standard http (webdav))
        Given the method is LOCK
        When filtered req(set(method, "LOCK"))
        Then method is LOCK

