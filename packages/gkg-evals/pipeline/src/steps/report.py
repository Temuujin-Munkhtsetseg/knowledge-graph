import json
from typing import List, Dict, Any

from utils import TomlConfig
from src.opencode.opencode import OpencodeRunSessionData
from src.opencode.models import AssistantMessage, StepFinishPart, Tokens, CacheTokens, MessageOrPart
from src.opencode.models import extract_assistant_messages, extract_user_messages, extract_parts
from dataclasses import dataclass, field
import uuid
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


@dataclass
class SessionStatistics:
    session_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    counts: Dict[str, Any] = field(default_factory=dict)
    cost: Dict[str, Any] = field(default_factory=dict)
    tokens: Dict[str, Any] = field(default_factory=dict)
    timing: Dict[str, Any] = field(default_factory=dict)
    is_agg: bool = False

    @classmethod
    def avg(cls, session_statistics: List['SessionStatistics']) -> 'SessionStatistics':
        if not session_statistics:
            return cls()
            
        avg_session_statistics = cls()
        n = len(session_statistics)
        
        # Helper function to average numeric values, skip non-numeric
        def avg_numeric_values(values):
            if not values:
                return 0
            numeric_values = [v for v in values if isinstance(v, (int, float))]
            return sum(numeric_values) / len(numeric_values) if numeric_values else 0
        
        # Helper function to merge dictionaries by averaging their numeric values
        def avg_dict_values(dicts):
            if not dicts:
                return {}
            all_keys = set()
            for d in dicts:
                if isinstance(d, dict):
                    all_keys.update(d.keys())
            
            result = {}
            for key in all_keys:
                values = [d.get(key, 0) for d in dicts if isinstance(d, dict)]
                result[key] = avg_numeric_values(values)
            return result
        
        # Average counts (handle nested dicts)
        avg_session_statistics.counts = {}
        if session_statistics[0].counts:
            for key in session_statistics[0].counts:
                values = [s.counts.get(key, 0) for s in session_statistics]
                if key in ["parts_by_type", "tools_used"]:
                    # These are nested dictionaries
                    avg_session_statistics.counts[key] = avg_dict_values(values)
                else:
                    # These are numeric values
                    avg_session_statistics.counts[key] = avg_numeric_values(values)
        
        # Average cost (all numeric)
        avg_session_statistics.cost = {}
        if session_statistics[0].cost:
            for key in session_statistics[0].cost:
                values = [s.cost.get(key, 0) for s in session_statistics]
                avg_session_statistics.cost[key] = avg_numeric_values(values)
        
        # Average tokens (all numeric)
        avg_session_statistics.tokens = {}
        if session_statistics[0].tokens:
            for key in session_statistics[0].tokens:
                values = [s.tokens.get(key, 0) for s in session_statistics]
                avg_session_statistics.tokens[key] = avg_numeric_values(values)
        
        # For timing, we'll skip averaging the nested list and just average the total
        avg_session_statistics.timing = {}
        if session_statistics[0].timing:
            for key in session_statistics[0].timing:
                if key == "assistant_messages_with_timing":
                    # Skip averaging the complex nested structure
                    avg_session_statistics.timing[key] = []
                else:
                    values = [s.timing.get(key, 0) for s in session_statistics]
                    avg_session_statistics.timing[key] = avg_numeric_values(values)
        
        avg_session_statistics.is_agg = True
        return avg_session_statistics

    def to_dict(self) -> Dict[str, Any]:
        return {
            "session_id": self.session_id,
            "counts": self.counts,
            "cost": self.cost,
            "tokens": self.tokens,
            "timing": self.timing,
            "is_agg": self.is_agg
        }

def get_session_statistics(session_data: OpencodeRunSessionData) -> SessionStatistics:
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
    
    return SessionStatistics(
        session_id=session_data.session_id,
        counts={
            "total_items": len(messages),
            "assistant_messages": len(assistant_messages),
            "user_messages": len(user_messages),
            "message_parts": len(parts),
            "parts_by_type": part_counts,
            "tools_used": tool_counts,
        },
        cost={
            "total": total_cost,
            "per_message": total_cost / max(len(assistant_messages), 1),
        },
        tokens={
            "input": total_tokens.input,
            "output": total_tokens.output,
            "reasoning": total_tokens.reasoning,
            "cache_read": total_tokens.cache.read,
            "cache_write": total_tokens.cache.write,
            "total": total_tokens.input + total_tokens.output + total_tokens.reasoning,
        },
        timing={
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
    )

def find_swe_bench_internal_report(toml_config: TomlConfig) -> dict | None:
    harness_location_dir = toml_config.pipeline.session_paths.swe_bench_harness_location_dir
    json_files = []

    time_created = 0
    
    # Find only top-level JSON files (not in subdirectories)
    for file in harness_location_dir.glob("*.json"):
        if file.is_file():
            json_files.append(file)
            time_created = max(time_created, file.stat().st_ctime)
    
    if not json_files:
        return None

    for file in json_files:
        if file.stat().st_ctime == time_created:
            with open(file, "r") as f:
                return json.load(f)
    return None

def generate_report(toml_config: TomlConfig):
    try:
        with open(toml_config.pipeline.session_paths.session_data_path, "r") as f:
            session_data = [dict(orjson.loads(line)) for line in f.readlines()]
        session_data = [OpencodeRunSessionData.from_dict(session) for session in session_data]
        session_stats = []
        for session in session_data:
            print(f"--------------------------------")
            print(f"Generating report for {session.fixture.instance_id}")
            report = get_session_statistics(session)
            print(json.dumps(report.to_dict(), indent=4))
            print(f"--------------------------------")
            session_stats.append(report)

        print(f"Generating average report for all sessions")
        avg_session_statistics = SessionStatistics.avg(session_stats)
        avg_session_statistics.session_id = toml_config.pipeline.session_name
        print(json.dumps(avg_session_statistics.to_dict(), indent=4))

        report_path = toml_config.pipeline.session_paths.swe_bench_report_path
        with open(report_path, "w") as f:
            report = {
                "stats": [s.to_dict() for s in session_stats],
                "avg_stats": avg_session_statistics.to_dict(),
                "swe_bench_internal_report": find_swe_bench_internal_report(toml_config),
            }
            report_as_json = json.dumps(report, indent=4)
            print(report_as_json)
            f.write(report_as_json)
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(e)
