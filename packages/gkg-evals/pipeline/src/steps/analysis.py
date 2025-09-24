from src.utils import TomlConfig
from src.cross_run_analysis.analysis import analyze_cross_run
from src.cross_run_analysis.plotting import plot_cross_run_results
from src.cross_run_analysis.plotting import tool_usage_comparison_chart
from src.cross_run_analysis.plotting import fixture_passes_chart
from src.cross_run_analysis.plotting import file_access_metrics_chart
# from src.cross_run_analysis.plotting import file_access_timeline_chart

PINNED_RUN = "2025-09-16--10:45:24"
# Used the pinned run to generate results for a single archive
# Else it will be an arithmetic average of all archives, split by pipeline run name

def cross_run_analysis(toml_config: TomlConfig):
    # metadata = analyze_cross_run(pinned_run=PINNED_RUN)
    metadata = analyze_cross_run(pinned_run=None)
    plot_cross_run_results(metadata)
    tool_usage_comparison_chart(metadata)
    fixture_passes_chart(metadata)
    # file_access_timeline_chart(metadata)
    file_access_metrics_chart(metadata)