module Authenticatable
  def authenticate(token)
    # verify token
  end

  def current_user
    @current_user
  end
end

module Serializable
  def to_json
    # serialize to JSON
  end

  def from_json(data)
    # deserialize from JSON
  end
end
