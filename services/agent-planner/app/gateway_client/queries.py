GET_USER_CONNECTORS = """
query GetUserConnectors($groupId: String) {
  getUserConnectors(groupId: $groupId) {
    id
    userId
    groupId
    kind
    authMethod
    status
    errorMessage
    accessToken
    createdAt
    updatedAt
    lastSyncAt
  }
}
"""

GET_SELECTED_RESOURCES = """
query GetSelectedResources($connectorId: String!) {
  getSelectedResources(connectorId: $connectorId) {
    id
    connectorId
    resourceKind
    externalId
    name
    description
    selected
    lastSyncAt
    nodeCount
    createdAt
    meta
  }
}
"""

GET_DOCUMENTS = """
query GetDocuments($zone: Int, $workspaceId: String) {
  documents(zone: $zone, workspaceId: $workspaceId) {
    id
    title
    kind
    domain
    excerpt
    zone
  }
}
"""

GET_NODES = """
query GetNodes($domain: String, $zone: Int, $workspaceId: String) {
  nodes(domain: $domain, zone: $zone, workspaceId: $workspaceId) {
    id
    domain
    sourceUri
    sourceText
    kind
    metadata {
      key
      value
    }
    zone
    embeddingDimensions
    isEmbedded
  }
}
"""

GET_WORKSPACE_SHARES = """
query WorkspaceShares($workspaceId: String!) {
  workspaceShares(workspaceId: $workspaceId) {
    userId
    email
    displayName
  }
}
"""

GET_PLANNER_CHAT_HISTORY = """
query PlannerChatHistory($workspaceId: String!) {
  plannerChatHistory(workspaceId: $workspaceId) {
    sender
    content
    createdAt
  }
}
"""

ADD_PLANNER_CHAT_MESSAGE = """
mutation AddPlannerChatMessage($workspaceId: String!, $sender: String!, $content: String!) {
  addPlannerChatMessage(workspaceId: $workspaceId, sender: $sender, content: $content) {
    id
    sender
    content
  }
}
"""

CLEAR_PLANNER_CHAT_HISTORY = """
mutation ClearPlannerChatHistory($workspaceId: String!) {
  clearPlannerChatHistory(workspaceId: $workspaceId)
}
"""

GET_AVAILABLE_TOOLS = """
query GetAvailableTools {
  availableTools {
    id
    name
    description
    category
    parameters {
      name
      paramType
      description
      required
      defaultValue
    }
  }
}
"""

GET_USER_PROFILE = """
query GetUser($id: String!) {
  getUser(id: $id) {
    id
    displayName
    email
    companyId
    role
    policies
    assignedAgentRoles
    twinManifest
  }
}
"""

GET_COMPANY_USERS = """
query GetCompanyUsers($companyId: String!) {
  getCompanyUsers(companyId: $companyId) {
    id
    displayName
    email
    role
    policies
    assignedAgentRoles
    twinManifest
  }
}
"""

GET_COMPANY_TEAMS = """
query GetCompanyTeams($companyId: String!) {
  getCompanyTeams(companyId: $companyId) {
    id
    name
    description
  }
}
"""

GET_TEAM_MEMBERS = """
query GetTeamMembers($teamId: String!) {
  getTeamMembers(teamId: $teamId) {
    id
    teamId
    userId
    role
  }
}
"""

GET_TEAM_WORKFLOWS = """
query GetTeamWorkflows($teamId: String!) {
  getTeamWorkflows(teamId: $teamId) {
    id
    teamId
    name
    description
    steps
    isEnabled
  }
}
"""

