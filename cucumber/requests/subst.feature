Feature: substitution action
    Scenario: system command
        Given a header user-agent is curl
        When filtered req(sub(header("user-agent"), "tr c h")) 
        Then header user-agent is hurl

    Scenario: basic function
        Given the method is GET
        When filtered req(sub(method, function(s) return "POST" end))
        Then method is POST

    Scenario: more in depth function
        Given a header user-agent is curl
        When handled by
            """
            local function add_version(name)
                return name .. '/0.0.0-example'
            end

            target("example.com:3000")
                :req(sub(header("user-agent"), add_version))
            """
        Then header user-agent is curl/0.0.0-example


