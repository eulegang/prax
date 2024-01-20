--focus()

target("google.com:443")

target('localhost:8000')
    :req(
      set(header("Authorization"), "Bearer abc"),
      set(query("xyz"), "true")
    )
    :resp(
      set(header("X-AttackProxy"), "set")
    )
