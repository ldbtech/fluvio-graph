import strawberry
from strawberry.federation import Schema
from src.graphql.query import Query
from src.graphql.mutation import Mutation

schema = Schema(query=Query, mutation=Mutation)
