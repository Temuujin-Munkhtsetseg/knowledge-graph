import json
import math
import re
from pathlib import Path
from dataclasses import dataclass
from collections import defaultdict
from typing import List, Set

from src.opencode.opencode import OpencodeRunSessionData
from src.harness.swe_bench import SweBenchCorrectPatch
from src.opencode.models import extract_tool_calls, ToolStateCompleted

from src.constants import ARCHIVE_DIR, SWEBENCH_REPORT_PATH, SESSION_DATA_PATH

WORKTREES_PATH_PATTERN = r'.*\/worktrees\/.*'

def flatten(l: list) -> list:
    return [item for sublist in l for item in sublist]

def parse_diff_file_paths(diff_content: str) -> List[str]:
    """
    Parse file paths from git diff format.
    
    Args:
        diff_content: The diff content in git format
        
    Returns:
        List of file paths found in the diff
    """
    file_paths = []
    
    # Look for lines starting with "diff --git", "+++", or "---"
    # Format: diff --git a/path/to/file b/path/to/file
    # Format: +++ b/path/to/file
    # Format: --- a/path/to/file
    
    diff_patterns = [
        r'^diff --git a/(.+?) b/(.+?)$',  # Extract both a/ and b/ paths
        r'^\+\+\+ b/(.+?)$',  # New file path
        r'^--- a/(.+?)$',     # Old file path
    ]
    
    for line in diff_content.split('\n'):
        line = line.strip()
        for pattern in diff_patterns:
            match = re.match(pattern, line)
            if match:
                if pattern.startswith(r'^\+\+\+') or pattern.startswith(r'^---'):
                    # Single path patterns
                    file_paths.append(match.group(1))
                else:
                    # diff --git pattern with two paths
                    file_paths.extend([match.group(1), match.group(2)])
    
    # Remove duplicates while preserving order
    seen = set()
    unique_paths = []
    for path in file_paths:
        if path not in seen:
            seen.add(path)
            unique_paths.append(path)
    
    return unique_paths


def find_matching_worktree_paths(diff_paths: List[str], file_access_order: List[str]) -> List[str]:
    """
    Find worktree paths in file_access_order that match the diff paths.
    
    Args:
        diff_paths: List of file paths from the diff
        file_access_order: List of accessed file paths (may contain worktree paths)
        
    Returns:
        List of matching worktree paths
    """
    matching_paths = []
    
    for diff_path in diff_paths:
        # Look for file_access_order entries that end with the diff_path
        for access_path in file_access_order:
            if '/worktrees/' in access_path and access_path.endswith(diff_path):
                matching_paths.append(access_path)
                break  # Only take the first match for each diff_path
    
    return matching_paths


@dataclass
class CrossRunMetadata:
    run_name: str 
    avg_duration_in_minutes: float
    resolved_instances_counts: dict[str, int]
    total_tools_used: int
    avg_tokens: float
    avg_tools_used: int
    tool_proportions: dict
    pass_rate: float
    timeout_rate: float
    tool_proportions_log: dict
    sum_log_proportions: float
    original_proportions: dict
    tools_used_dict: dict
    pass_counts: dict[str, int]
    session_data: list[OpencodeRunSessionData]
    is_agg: bool = False

    def pprint(self):
        print("--------------------------------")
        print(f"{self.run_name} pass counts: {json.dumps(self.pass_counts, indent=4)}")
        print(f"{self.run_name} pass rate: {self.pass_rate}%")
        print(f"{self.run_name} timeout rate: {self.timeout_rate}%")
        print(f"{self.run_name} avg duration: {self.avg_duration_in_minutes} minutes")
        print(f"{self.run_name} avg tokens: {self.avg_tokens} kTokens")
        print(f"{self.run_name} total tools used: {self.total_tools_used}")
        print(f"{self.run_name} resolved instances counts: {json.dumps(self.resolved_instances_counts, indent=4)}")
        print(f"{self.run_name}: {self.avg_tools_used} avg tools used")
        print("Tool uses proportions (log10 scale for better visibility):")
        print(f"{self.run_name}: {json.dumps(self.tool_proportions, indent=4)}")
        print(f"{self.run_name}: {sum(self.tool_proportions.values())} sum log proportions")
        print("Tool uses proportions (original %):")
        print(f"{self.run_name}: {json.dumps(self.original_proportions, indent=4)}")
        print(f"tools used dict: {json.dumps(self.tools_used_dict, indent=4)}")
        print("--------------------------------")

    @classmethod
    def avg(cls, cross_run_metadata: list['CrossRunMetadata']) -> 'CrossRunMetadata':
        head = cross_run_metadata[0]

        # Collect all unique keys from all instances to handle missing keys
        all_tool_proportion_keys = set()
        all_tool_proportion_log_keys = set()
        all_original_proportion_keys = set()
        all_tools_used_dict_keys = set()
        all_resolved_instances_counts_keys = set()
        all_pass_counts_keys = set()
        
        for item in cross_run_metadata:
            all_tool_proportion_keys.update(item.tool_proportions.keys())
            all_tool_proportion_log_keys.update(item.tool_proportions_log.keys())
            all_original_proportion_keys.update(item.original_proportions.keys())
            all_tools_used_dict_keys.update(item.tools_used_dict.keys())
            all_resolved_instances_counts_keys.update(item.resolved_instances_counts.keys())
            all_pass_counts_keys.update(item.pass_counts.keys())

        # Sum the resolved instances counts
        resolved_instances_counts = {k: sum(item.resolved_instances_counts.get(k, 0) for item in cross_run_metadata) for k in all_resolved_instances_counts_keys}

        return cls(
            run_name=head.run_name,
            avg_duration_in_minutes=sum(item.avg_duration_in_minutes for item in cross_run_metadata) / len(cross_run_metadata),
            resolved_instances_counts=resolved_instances_counts,
            total_tools_used=sum(item.total_tools_used for item in cross_run_metadata),
            avg_tokens=sum(item.avg_tokens for item in cross_run_metadata) / len(cross_run_metadata),
            avg_tools_used=sum(item.avg_tools_used for item in cross_run_metadata) / len(cross_run_metadata),
            tool_proportions={k: sum(item.tool_proportions.get(k, 0) for item in cross_run_metadata) / len(cross_run_metadata) for k in all_tool_proportion_keys},
            tool_proportions_log={k: sum(item.tool_proportions_log.get(k, 0) for item in cross_run_metadata) / len(cross_run_metadata) for k in all_tool_proportion_log_keys},
            sum_log_proportions=sum(item.sum_log_proportions for item in cross_run_metadata),
            original_proportions={k: sum(item.original_proportions.get(k, 0) for item in cross_run_metadata) / len(cross_run_metadata) for k in all_original_proportion_keys},
            tools_used_dict={k: sum(item.tools_used_dict.get(k, 0) for item in cross_run_metadata) / len(cross_run_metadata) for k in all_tools_used_dict_keys},
            timeout_rate=sum(item.timeout_rate for item in cross_run_metadata) / len(cross_run_metadata),
            pass_rate=sum(item.pass_rate for item in cross_run_metadata) / len(cross_run_metadata),
            pass_counts={k: sum(item.pass_counts.get(k, 0) for item in cross_run_metadata) for k in all_pass_counts_keys},
            session_data = flatten([sd.session_data for sd in cross_run_metadata]),
            is_agg=True,
        )
        

def parse_data(path: Path) -> tuple[dict, list[OpencodeRunSessionData], float]:
    report_path = path / SWEBENCH_REPORT_PATH
    with open(report_path, "r") as f:
        report = json.load(f)
    session_data_path = path / SESSION_DATA_PATH
    with open(session_data_path, "r") as f:
        session_data = [json.loads(line) for line in f.readlines()]
        session_data = [OpencodeRunSessionData.from_dict(session) for session in session_data]
    
    for session in session_data:
        tool_calls = extract_tool_calls(session.messages)
        for tool_call in tool_calls:
            match tool_call.tool:
                case "read" | "edit" | "grep" | "glob":
                    input_keys = set([k for k in tool_call.state.input.keys()])
                    for word in input_keys:
                        for path_word in ["path", "paths", "file", "files"]:
                            if path_word in word.lower():
                                session.file_access_order.append(tool_call.state.input[word])
                                break
                case "knowledge-graph_list_projects" | "knowledge-graph_index_project" | "knowledge-graph_repo_map":
                    continue
                case _:
                    # Filter to only include paths that actually contain /worktrees/
                    extracted_paths = re.findall(WORKTREES_PATH_PATTERN, tool_call.state.output)
                    if extracted_paths:
                        session.file_access_order.extend(extracted_paths)

        # Parse file paths from the diff and match them with worktree paths
        diff_paths = parse_diff_file_paths(session.patch.model_patch)
        matching_worktree_paths = find_matching_worktree_paths(diff_paths, session.file_access_order)
        session.patch_paths.extend(matching_worktree_paths)

    # for session in session_data:
    #     print(f"session: {session.session_id}, file access order: {session.file_access_order}")
    #     print(f"session: {session.session_id}, patch paths: {session.patch_paths}")
    #     print("--------------------------------")

    return report, session_data

def analyze_cross_run(pinned_run: str = None) -> dict[str, CrossRunMetadata]:
    archive_dirs = []
    for archive_dir in ARCHIVE_DIR.glob("*"):
        if pinned_run and archive_dir.name != pinned_run:
            continue
        archive_dirs.append(archive_dir.glob("*"))

    archive_paths : list[Path] = []
    for archive_path in archive_dirs:
        for archive_dir in archive_path:
            if archive_dir.is_dir():
                for sub_dir in archive_dir.iterdir():
                    if sub_dir.is_dir():
                        archive_paths.append(sub_dir)

    for p in archive_paths:
        print(p)
        
    cross_run_metadata = defaultdict(list)
    for path in archive_paths:
        report, session_data = parse_data(path)
        timeout_rate = sum(1 for session in session_data if session.killed and session.killed_reason == "timeout") / len(session_data) * 100
        timeout_rate = round(timeout_rate, 1)

        # NOTE: Use this for totals!
        session_stats_from_report = report["stats"]
        total_tools_used = 0
        for session_stat in session_stats_from_report:
            tool_counts = session_stat["counts"]["tools_used"]
            total_tools_used += sum(tool_counts.values())
            """
            {'session_id': 'ses_6af67a546ffeYG06BIKlzOF00P', 'counts': {'total_items': 126, 'assistant_messages': 1, 'user_messages': 1, 'message_parts': 124, 'parts_by_type': {'step-finish': 29, 'text': 24, 'tool': 28, 'step-start': 29, 'patch': 14}, 'tools_used': {'read': 11, 'grep': 8, 'todowrite': 4, 'knowledge-graph_search_codebase_definitions': 1, 'knowledge-graph_list_projects': 1, 'edit': 2, 'knowledge-graph_read_definitions': 1}}, 'cost': {'total': 0.7793874000000002, 'per_message': 0.7793874000000002}, 'tokens': {'input': 20, 'output': 6585, 'reasoning': 0, 'cache_read': 660029, 'cache_write': 29068, 'total': 6605}, 'timing': {'assistant_messages_with_timing': [{'id': 'msg_950985b2200126Qv4TRNXB8uJi', 'created': 1757993786146, 'completed': 1757993966196, 'duration_ms': 180050}], 'total_duration_ms': 180050}, 'is_agg': False}
            """
            # print("TEST_-----------------")
        
        swe_bench_internal_report = report["swe_bench_internal_report"]
        resolved_instances = swe_bench_internal_report["resolved_instances"]
        resolved_ids = swe_bench_internal_report["resolved_ids"]
        resolved_instances_counts = {k: 1 for k in resolved_ids}
        pass_counts = {resolved_instances : 1}
        pass_rate = (resolved_instances / swe_bench_internal_report["total_instances"]) * 100
        pass_rate = round(pass_rate, 1)
        avg_stats = report["avg_stats"]
        avg_duration_in_minutes = avg_stats["timing"]["total_duration_ms"] / 1000 / 60
        avg_duration_in_minutes = round(avg_duration_in_minutes, 1)
        avg_tokens = round(sum(avg_stats["tokens"].values()) / 1000, 1)
        tools_used_dict = avg_stats["counts"]["tools_used"]
        # drop the invalid key
        if "invalid" in tools_used_dict:
            tools_used_dict.pop("invalid")
        tools_used = round(sum(tools_used_dict.values()), 1)

        # Calculate log-based proportions for better visualization of small values
        tool_proportions_log = {}
        for k, v in tools_used_dict.items():
            proportion = v / tools_used * 100
            # Use log10(proportion + 1) to handle 0 values and compress the scale
            log_proportion = math.log10(proportion + 1) if proportion > 0 else 0
            tool_proportions_log[k] = round(log_proportion, 3)

        sum_log_proportions = sum(tool_proportions_log.values())
        tool_proportions = {k: round(v / sum_log_proportions * 100, 1) for k, v in tool_proportions_log.items()}
        # Also keep original proportions for reference
        original_proportions = {k: round(v / tools_used * 100, 1) for k, v in tools_used_dict.items()}

        cross_run_metadata_item = CrossRunMetadata(
            run_name=path.name,
            avg_duration_in_minutes=avg_duration_in_minutes,
            resolved_instances_counts=resolved_instances_counts,
            avg_tokens=avg_tokens,
            total_tools_used=total_tools_used,
            avg_tools_used=tools_used,
            tools_used_dict=tools_used_dict,
            tool_proportions=tool_proportions,
            tool_proportions_log=tool_proportions_log,
            sum_log_proportions=sum_log_proportions,
            original_proportions=original_proportions,
            timeout_rate=timeout_rate,
            pass_rate=pass_rate,
            pass_counts=pass_counts,
            session_data=session_data,
        )
        cross_run_metadata[path.name].append(cross_run_metadata_item)

    if not pinned_run:
        for k, v in cross_run_metadata.items():
            print(f"{k}: {len(v)}")
            avg_run = CrossRunMetadata.avg(v)
            avg_run.pprint()
            cross_run_metadata[k] = [avg_run]

    return {k: v[0] for k, v in cross_run_metadata.items()}

