focus()

target('localhost:8000')
    :req(
      set_header("Authorization", "Bearer xyz"),
      dump,
      intercept
    )
    :resp(
      set_header("X-AttackProxy", "set"),
      dump
    )

target('localhost:3000')
    :req(
      set_header("Authorization", "Bearer xyz"),
      dump,
      intercept
    )
    :resp(
      set_header("X-AttackProxy", "set"),
      dump
    )
