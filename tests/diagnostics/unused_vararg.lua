function Foo(...)
  -- unused vararg
end

function Foo(...t)
  -- unused vararg
end

function Foo(...)
  print(...)
end

function Foo(...t)
  function Bar()
    print(t)
  end
end
