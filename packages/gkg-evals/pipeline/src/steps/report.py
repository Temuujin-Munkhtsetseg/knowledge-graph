import json
from typing import List, Dict, Any

from utils import TomlConfig
from src.opencode.opencode import OpencodeRunSessionData
from src.opencode.models import AssistantMessage, StepFinishPart, Tokens, CacheTokens, MessageOrPart
from src.opencode.models import extract_assistant_messages, extract_user_messages, extract_parts

import orjson

from src.constants import SESSION_DATA_PATH, SWEBENCH_REPORT_DIR

def calculate_total_cost(messages: List[MessageOrPart]) -> float:
    """
    Calculate the total cost from all assistant messages and step-finish parts.
    
    Args:
        metrics: Parsed SessionMetrics object
        
    Returns:
        float: Total cost
    """
    total_cost = 0.0
    
    for item in messages:
        if isinstance(item, AssistantMessage):
            total_cost += item.cost
        elif isinstance(item, StepFinishPart):
            total_cost += item.cost
    
    return total_cost


def calculate_total_tokens(messages: List[MessageOrPart]) -> Tokens:
    """
    Calculate the total token usage from all assistant messages and step-finish parts.
    
    Args:
        metrics: Parsed SessionMetrics object
        
    Returns:
        Tokens: Aggregated token usage
    """
    total_input = 0
    total_output = 0
    total_reasoning = 0
    total_cache_read = 0
    total_cache_write = 0
    
    for item in messages:
        if isinstance(item, AssistantMessage):
            total_input += item.tokens.input
            total_output += item.tokens.output
            total_reasoning += item.tokens.reasoning
            total_cache_read += item.tokens.cache.read
            total_cache_write += item.tokens.cache.write
        elif isinstance(item, StepFinishPart):
            total_input += item.tokens.input
            total_output += item.tokens.output
            total_reasoning += item.tokens.reasoning
            total_cache_read += item.tokens.cache.read
            total_cache_write += item.tokens.cache.write
    
    return Tokens(
        input=total_input,
        output=total_output,
        reasoning=total_reasoning,
        cache=CacheTokens(read=total_cache_read, write=total_cache_write)
    )


def get_session_statistics(session_data: OpencodeRunSessionData) -> Dict[str, Any]:
    """
    Get comprehensive statistics about a session.
    
    Args:
        metrics: Parsed SessionMetrics object
        
    Returns:
        Dict[str, Any]: Dictionary containing various statistics
    """
    messages = session_data.messages
    assistant_messages = extract_assistant_messages(messages)
    user_messages = extract_user_messages(messages)
    parts = extract_parts(messages)
    
    # Count parts by type
    part_counts = {}
    for part in parts:
        part_type = getattr(part, 'type', 'unknown')
        part_counts[part_type] = part_counts.get(part_type, 0) + 1
    
    # Count tool usage
    tool_counts = {}
    for part in parts:
        if hasattr(part, 'type') and part.type == 'tool':
            tool_name = getattr(part, 'tool', 'unknown')
            tool_counts[tool_name] = tool_counts.get(tool_name, 0) + 1
    
    total_cost = calculate_total_cost(messages)
    total_tokens = calculate_total_tokens(messages)
    
    return {
        "session_id": session_data.session_id,
        "counts": {
            "total_items": len(messages),
            "assistant_messages": len(assistant_messages),
            "user_messages": len(user_messages),
            "message_parts": len(parts),
            "parts_by_type": part_counts,
            "tools_used": tool_counts,
        },
        # "cost": {
        #     "total": total_cost,
        #     "per_message": total_cost / max(len(assistant_messages), 1),
        # },
        "tokens": {
            "input": total_tokens.input,
            "output": total_tokens.output,
            "reasoning": total_tokens.reasoning,
            "cache_read": total_tokens.cache.read,
            "cache_write": total_tokens.cache.write,
            "total": total_tokens.input + total_tokens.output + total_tokens.reasoning,
        },
        "timing": {
            "assistant_messages_with_timing": [
                {
                    "id": msg.id,
                    "created": msg.time.created,
                    "completed": msg.time.completed,
                    "duration_ms": (msg.time.completed - msg.time.created) if msg.time.completed else None,
                }
                for msg in assistant_messages
            ],
            "total_duration_ms": sum(msg.time.completed - msg.time.created for msg in assistant_messages if msg.time.completed),
        }
    }

def generate_report(toml_config: TomlConfig):
    with open(SESSION_DATA_PATH, "r") as f:
        session_data = [dict(orjson.loads(line)) for line in f.readlines()]
    session_data = [OpencodeRunSessionData.from_dict(session) for session in session_data]
    for session in session_data:
        report = get_session_statistics(session)
        print(json.dumps(report, indent=4))

    # with open(SWEBENCH_REPORT_DIR / "report.json", "r") as f:
    #     json.dump(metrics, f, indent=4)
