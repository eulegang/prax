--focus()

target('localhost:8000')
    :req(
      set_header("Authorization", "Bearer xyz"),
      dump
    )
    :resp(
      set_header("X-AttackProxy", "set"),
      dump
    )
