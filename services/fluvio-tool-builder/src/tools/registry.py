import os
import json
import logging
import importlib
from typing import List, Dict, Any
from pydantic import BaseModel
from src.graphql.types import GqlTool, GqlToolParameter

logger = logging.getLogger("tools-registry")

class DynamicRegistry:
    def __init__(self):
        self.tools_dir = os.path.dirname(os.path.abspath(__file__))

    def get_all_tools(self) -> List[GqlTool]:
        """Scan tools/ directories and load manifest.json metadata dynamically."""
        tools = []
        try:
            for item in os.listdir(self.tools_dir):
                item_path = os.path.join(self.tools_dir, item)
                if os.path.isdir(item_path):
                    manifest_path = os.path.join(item_path, "manifest.json")
                    if os.path.exists(manifest_path):
                        try:
                            with open(manifest_path, "r") as f:
                                data = json.load(f)
                            
                            params = []
                            for p in data.get("parameters", []):
                                params.append(GqlToolParameter(
                                    name=p.get("name"),
                                    param_type=p.get("paramType", "String"),
                                    description=p.get("description", ""),
                                    required=p.get("required", False),
                                    default_value=p.get("defaultValue")
                                ))

                            tools.append(GqlTool(
                                id=data.get("id"),
                                name=data.get("name"),
                                description=data.get("description"),
                                category=data.get("category", "general"),
                                is_data_science=data.get("is_data_science", False),
                                parameters=params
                            ))
                        except Exception as e:
                            logger.error(f"Failed to load manifest for tool '{item}': {e}")
        except Exception as e:
            logger.error(f"Error scanning tools directory: {e}")
        return tools

    async def execute_tool_action(self, tool_id: str, inputs_json: str, log_stream: List[str]) -> Dict[str, Any]:
        """Dynamically load runtime class and execute the requested action."""
        try:
            inputs = json.loads(inputs_json)
        except Exception as e:
            raise Exception(f"Failed to parse inputs JSON: {e}")

        action = inputs.get("action")
        arguments_raw = inputs.get("arguments")

        if not action:
            raise Exception("No 'action' specified in inputs.")

        log_stream.append(f"Resolving runtime module for tool: {tool_id}")
        # 1. Dynamically import runtime module (using snake_case for python package compatibility)
        safe_tool_id = tool_id.replace("-", "_")
        try:
            runtime_module = importlib.import_module(f"src.tools.{safe_tool_id}.runtime")
            contracts_module = importlib.import_module(f"src.tools.{safe_tool_id}.contracts")
        except ModuleNotFoundError as e:
            raise Exception(f"Runtime or Contract module for '{tool_id}' (package '{safe_tool_id}') not found: {e}")

        # 2. Find the runtime class (converting snake_case to PascalCase, e.g. data_cleaning -> DataCleaningRuntime)
        class_name = "".join(word.capitalize() for word in safe_tool_id.split("_")) + "Runtime"
        runtime_class = getattr(runtime_module, class_name, None)
        if not runtime_class:
            raise Exception(f"Class '{class_name}' not found in runtime module.")

        instance = runtime_class()
        method = getattr(instance, action, None)
        if not method:
            raise Exception(f"Action '{action}' is not supported by tool '{tool_id}'.")

        # 3. Parse arguments into Pydantic models using metadata from method signature
        log_stream.append(f"Preparing arguments for action: {action}")
        try:
            arguments = arguments_raw or {}
            while isinstance(arguments, str):
                arguments = json.loads(arguments)
            if not isinstance(arguments, dict):
                arguments = {}
        except Exception as e:
            raise Exception(f"Failed to parse arguments JSON: {e}")

        # Map arguments to signatures using type hints
        import inspect
        sig = inspect.signature(method)
        kwargs = {}
        for param_name, param in sig.parameters.items():
            if param_name in ["self", "args", "kwargs"]:
                continue
            
            # Lookup in arguments
            val = arguments.get(param_name)
            
            # Resolve type annotation (e.g. KafkaExecutionContext, KafkaTopicConfig)
            annotation = param.annotation
            
            if annotation and inspect.isclass(annotation) and issubclass(annotation, BaseModel):
                # Instantiate Pydantic model from dictionary
                if val is None:
                    # Let validation trigger or pass empty dict
                    kwargs[param_name] = annotation()
                else:
                    kwargs[param_name] = annotation(**val)
            else:
                kwargs[param_name] = val

        # 4. Invoke method
        log_stream.append(f"Executing action: {action}")
        try:
            result = await method(**kwargs)
            log_stream.append(f"Action {action} completed successfully.")
            
            # Serialize result if it contains Pydantic models
            if isinstance(result, list):
                serialized_result = []
                for item in result:
                    if isinstance(item, BaseModel):
                        serialized_result.append(item.model_dump())
                    else:
                        serialized_result.append(item)
            elif isinstance(result, BaseModel):
                serialized_result = result.model_dump()
            else:
                serialized_result = result

            # Honor a runtime that signals failure via its return value (many
            # runtimes return {"status": "failed", "error": ...} instead of
            # raising). Without this, an honest failure is masked as success.
            if isinstance(serialized_result, dict) and serialized_result.get("status") == "failed":
                err = serialized_result.get("error") or "Tool reported failure without detail."
                log_stream.append(f"Action {action} reported failure: {err}")
                return {"status": "failed", "error": err}

            # A bare False/None from a boolean-returning action means the operation
            # did not succeed — never report that as success.
            if serialized_result is False or serialized_result is None:
                msg = f"Action '{action}' on tool '{tool_id}' returned no success result."
                log_stream.append(msg)
                return {"status": "failed", "error": msg}

            return {
                "status": "success",
                "result": serialized_result
            }
        except Exception as e:
            log_stream.append(f"Error during execution: {e}")
            logger.error(f"Error executing tool action {tool_id}/{action}: {e}", exc_info=True)
            return {
                "status": "failed",
                "error": str(e)
            }

registry = DynamicRegistry()
