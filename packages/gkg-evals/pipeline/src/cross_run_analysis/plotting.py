from src.utils import TomlConfig
import plotly.graph_objects as go
from plotly.subplots import make_subplots
from src.cross_run_analysis.analysis import CrossRunMetadata
import base64
import os

DEFAULT_PLOT_WIDTH = 3840
DEFAULT_PLOT_HEIGHT = 2160

# Name mapping for cleaner display
NAME_MAP = {
    "baseline": "Grep Tools",
    "gkg_only": "GKG Only", 
    "baseline_with_gkg": "GKG + Grep Tools"
}

# Tool name mapping for cleaner display
TOOL_NAME_MAP = {
    "read": "Read File",
    "grep": "Grep Search", 
    "edit": "Edit File",
    "todowrite": "Todo Write",
    "knowledge-graph_search_codebase_definitions": "KG: Search Definitions",
    "knowledge-graph_read_definitions": "KG: Read Definitions",
    "knowledge-graph_list_projects": "KG: List Projects",
    "knowledge-graph_repo_map": "KG: Repo Map",
    "knowledge-graph_get_references": "KG: Get References",
    "glob": "Glob Search",
    "codebase_search": "Codebase Search"
}

def create_gitlab_font_css():
    """Create CSS with embedded GitLab fonts"""
    font_dir = os.path.join(os.path.dirname(__file__), '..', '..', 'assets', 'fonts')
    
    def encode_font(font_path):
        """Encode font file to base64"""
        if os.path.exists(font_path):
            with open(font_path, 'rb') as font_file:
                return base64.b64encode(font_file.read()).decode()
        return None
        
    # Encode GitLab fonts
    gitlab_sans = encode_font(os.path.join(font_dir, 'GitLabSans.otf'))
    gitlab_sans_italic = encode_font(os.path.join(font_dir, 'GitLabSans-Italic.otf'))
    gitlab_mono = encode_font(os.path.join(font_dir, 'GitLabMono.ttf'))
    gitlab_mono_italic = encode_font(os.path.join(font_dir, 'GitLabMono-Italic.ttf'))
    
    css_parts = ['<style>']
    
    # GitLab Sans Regular
    if gitlab_sans:
        css_parts.append(f"""
        @font-face {{
            font-family: 'GitLab Sans';
            src: url(data:font/opentype;charset=utf-8;base64,{gitlab_sans}) format('opentype');
            font-weight: normal;
            font-style: normal;
            font-display: swap;
        }}""")
    
    # GitLab Sans Italic
    if gitlab_sans_italic:
        css_parts.append(f"""
        @font-face {{
            font-family: 'GitLab Sans';
            src: url(data:font/opentype;charset=utf-8;base64,{gitlab_sans_italic}) format('opentype');
            font-weight: normal;
            font-style: italic;
            font-display: swap;
        }}""")
    
    # GitLab Mono Regular
    if gitlab_mono:
        css_parts.append(f"""
        @font-face {{
            font-family: 'GitLab Mono';
            src: url(data:font/truetype;charset=utf-8;base64,{gitlab_mono}) format('truetype');
            font-weight: normal;
            font-style: normal;
            font-display: swap;
        }}""")
    
    # GitLab Mono Italic
    if gitlab_mono_italic:
        css_parts.append(f"""
        @font-face {{
            font-family: 'GitLab Mono';
            src: url(data:font/truetype;charset=utf-8;base64,{gitlab_mono_italic}) format('truetype');
            font-weight: normal;
            font-style: italic;
            font-display: swap;
        }}""")
    
    css_parts.append('</style>')
    return '\n'.join(css_parts)

def export_plot(fig: go.Figure, show: bool = True, export_path: str = None):
    # Generate GitLab fonts CSS
    gitlab_css = create_gitlab_font_css()
    
    # Show with embedded fonts by temporarily writing HTML and opening
    import tempfile
    import webbrowser
    import os
    
    # Create HTML with embedded fonts
    html_content = fig.to_html(include_plotlyjs=True)
    html_with_fonts = gitlab_css + html_content
    
    # For fig.show() - create temp file and open in browser
    with tempfile.NamedTemporaryFile(mode='w', suffix='.html', delete=False, encoding='utf-8') as temp_file:
        temp_file.write(html_with_fonts)
        temp_path = temp_file.name
    
    # Open in browser (this replaces fig.show())
    webbrowser.open(f'file://{os.path.abspath(temp_path)}')
    
    # # Export high-resolution PNG
    # try:
    #     output_path = "swebench_chart_4k.png"
    #     fig.write_image(output_path, width=3840, height=2160, scale=1)
    #     print(f"High-resolution chart saved to: {output_path}")
    # except Exception as e:
    #     print(f"Note: To save high-res images, install kaleido: pip install kaleido")
    #     print(f"Error: {e}")
    print("Cross run chart opened with embedded GitLab fonts!")


def plot_cross_run_results(cross_run_metadata: dict[str, CrossRunMetadata]):
    """Plot cross run results showing pass rates for each run"""
    
    # Extract and sort data by pass rate (descending)
    data_tuples = [(key, metadata.pass_rate, metadata.avg_duration_in_minutes) 
                   for key, metadata in cross_run_metadata.items()]
    data_tuples.sort(key=lambda x: x[1], reverse=False)  # Sort by pass rate descending
    
    run_names_raw = [item[0] for item in data_tuples]
    pass_rates = [item[1] for item in data_tuples]
    durations = [item[2] for item in data_tuples]
    run_names = [NAME_MAP[name] for name in run_names_raw]

    # Create figure
    fig = go.Figure()

    # Scale duration to match the visual proportions
    # Find max duration to normalize the scale
    max_duration = max(durations)
    # Scale durations to be proportional (normalize to max pass rate for visual balance)
    max_pass_rate = max(pass_rates)
    duration_scale_factor = max_pass_rate / max_duration * 0.6  # 0.6 to keep durations reasonable
    
    # Bottom bars represent duration (scaled)
    without_thinking = [duration * duration_scale_factor for duration in durations]
    # Top bars represent the remaining pass rate
    with_thinking = [pass_rate - duration_scaled for pass_rate, duration_scaled in zip(pass_rates, without_thinking)]
    
    # Add "Without thinking" bars (bottom layer)
    for i, (name, rate) in enumerate(zip(run_names, without_thinking)):
        if name == "GKG":
            # Solid color for GKG
            marker_style = dict(
                color='#FCA326',  # Pastel orange
                line=dict(width=0)
            )
        else:
            # Hollow with border for Baseline Tools and GKG + Baseline Tools
            marker_style = dict(
                color='#FCA326',
                line=dict(color='#FCA326', width=2)  # Pastel orange border
            )
        
        fig.add_trace(go.Bar(
            x=[name],
            y=[rate],
            name='Average Completion Time per Problem' if i == 0 else '',
            marker=marker_style,
            showlegend=True if i == 0 else False,
            legendgroup='without'
        ))
    
    # Add "With thinking" bars (top layer)
    for i, (name, rate) in enumerate(zip(run_names, with_thinking)):
        if name == "GKG":
            # Solid orange for GKG top layer
            marker_style = dict(
                color='#FC6D26',  # Solid orange
                line=dict(width=0)
            )
        else:
            # Hollow with border for Baseline Tools and GKG + Baseline Tools
            marker_style = dict(
                color='#FC6D26',
                line=dict(color='#FC6D26', width=2)  # Solid orange border
            )
        
        fig.add_trace(go.Bar(
            x=[name],
            y=[rate],
            name='SWE-Bench Score' if i == 0 else '',
            marker=marker_style,
            showlegend=True if i == 0 else False,
            legendgroup='with'
        ))
    
    # Add total percentage labels on top
    for i, (name, total_rate) in enumerate(zip(run_names, pass_rates)):
        fig.add_annotation(
            x=name,
            y=total_rate + 2.5,
            text=f'{total_rate:.1f}%',
            showarrow=False,
            font=dict(color='white', size=84, family='GitLab Sans, Arial, sans-serif', weight='bold')
        )
    
    # Add duration labels on top of pastel bars (bottom layer)
    for i, (name, pastel_rate, duration) in enumerate(zip(run_names, without_thinking, durations)):
        fig.add_annotation(
            x=name,
            y=pastel_rate - 2.5,
            text=f'{duration:.1f} min',
            showarrow=False,
            font=dict(color='#171321', size=63, family='GitLab Sans, Arial, sans-serif', weight=600)
        )

    # Update layout with improved styling
    fig.update_layout(
        title={
            'text': 'SWE-Bench-Lite<br><span style="font-size: 67px; font-weight: 400;">Dev Split, No Shell Access, No LSP Context, No Embeddings</span>',
            'font': {'size': 101, 'family': 'GitLab Sans, Arial, sans-serif', 'color': 'white', 'weight': 600},
            'x': 0.155,
            'y': 0.9
        },
        barmode='stack',
        barcornerradius=105,
        legend=dict(
            orientation="h",
            yanchor="top",
            y=1.1,
            xanchor="left",
            x=0,
            xref="paper",
            font=dict(size=50, family='GitLab Sans, Arial, sans-serif', color='white'),
            bgcolor='rgba(255,255,255,0)',
            borderwidth=0,
            itemwidth=150,
            # itemsizing='constant'
        ),
        yaxis=dict(
            title='Accuracy (%)',
            title_font=dict(size=76, family='GitLab Sans, Arial, sans-serif', color='white', weight=500),
            title_standoff=75,
            showticklabels=False,
            showgrid=False,
            zeroline=False,
            range=[0, max(pass_rates) * 1.1],  # Add more vertical padding between bars and title
        ),
        xaxis=dict(
            tickfont=dict(size=67, family='GitLab Sans, Arial, sans-serif', color='white'),
            showgrid=False,
        ),
        font=dict(size=50, family='GitLab Sans, Arial, sans-serif'),
        plot_bgcolor='#171321',
        paper_bgcolor='#171321',
        width=DEFAULT_PLOT_WIDTH,
        height=DEFAULT_PLOT_HEIGHT,
        margin=dict(l=504, r=168, t=504, b=504)  # Extra left margin to match reference style
    )
    export_plot(fig, show=True, export_path=None)

def tool_usage_comparison_chart(cross_run_metadata: dict[str, CrossRunMetadata]):
    """Create a horizontal stacked bar chart comparing tool usage across runs"""
    
    # Collect all unique tools across runs
    all_tools = set()
    for metadata in cross_run_metadata.values():
        all_tools.update(metadata.original_proportions.keys())
    
    # Sort tools by maximum usage across all runs for better ordering
    tool_max_usage = {}
    for tool in all_tools:
        max_usage = max(metadata.original_proportions.get(tool, 0) 
                       for metadata in cross_run_metadata.values())
        tool_max_usage[tool] = max_usage
    
    sorted_tools = sorted(all_tools, key=lambda x: tool_max_usage[x], reverse=True)
    
    # Color palette for different tools
    colors = [
        '#FC6D26',  # GitLab Orange
        '#6E49CB',  # GitLab Purple  
        '#1F75CB',  # GitLab Blue
        '#108548',  # GitLab Green
        '#C17D10',  # GitLab Yellow
        '#DD2B0E',  # GitLab Red
        '#5C4FE1',  # Light Purple
        '#428FDB',  # Light Blue
        '#37B24D',  # Light Green
        '#F59E0B',  # Light Orange
        '#E11D48',  # Light Red
        '#9C9C9C',  # Gray
    ]
    
    fig = go.Figure()
    
    # Create horizontal stacked bars
    y_labels = [NAME_MAP.get(name, name.replace('_', ' ').title()) for name in cross_run_metadata.keys()]
    
    for i, tool in enumerate(sorted_tools):
        values = []
        for run_name in cross_run_metadata.keys():
            value = cross_run_metadata[run_name].original_proportions.get(tool, 0)
            values.append(value)
        
        # Only show tools that have at least 1% usage in at least one run
        if max(values) >= 1.0:
            clean_tool_name = TOOL_NAME_MAP.get(tool, tool.replace('_', ' ').replace('-', ' ').title())
            
            fig.add_trace(go.Bar(
                name=clean_tool_name,
                y=y_labels,
                x=values,
                orientation='h',
                marker=dict(
                    color=colors[i % len(colors)],
                    line=dict(color='#171321', width=2)
                ),
                text=[f'{v:.1f}%' if v >= 3 else '' for v in values],  # Only show text for larger segments
                textposition='inside',
                textfont=dict(size=42, family='GitLab Sans, Arial, sans-serif', color='white', weight='bold'),
                hovertemplate='<b>%{fullData.name}</b><br>%{x:.1f}%<extra></extra>'
            ))
    
    # Update layout
    fig.update_layout(
        title={
            'text': 'Tool Usage Distribution Across Runs',
            'font': {'size': 101, 'family': 'GitLab Sans, Arial, sans-serif', 'color': 'white', 'weight': 600},
            'x': 0.5,
            'y': 0.95
        },
        barmode='stack',
        barcornerradius=15,
        xaxis=dict(
            title='Percentage of Tool Usage (%)',
            title_font=dict(size=76, family='GitLab Sans, Arial, sans-serif', color='white', weight=500),
            title_standoff=75,
            tickfont=dict(size=56, family='GitLab Sans, Arial, sans-serif', color='white'),
            showgrid=True,
            gridcolor='rgba(255,255,255,0.1)',
            range=[0, 100]
        ),
        yaxis=dict(
            tickfont=dict(size=67, family='GitLab Sans, Arial, sans-serif', color='white'),
            showgrid=False,
            categoryorder='array',
            categoryarray=y_labels[::-1]  # Reverse order so baseline is at top
        ),
        legend=dict(
            orientation="v",
            yanchor="top",
            y=0.98,
            xanchor="left", 
            x=1.02,
            font=dict(size=48, family='GitLab Sans, Arial, sans-serif', color='white'),
            bgcolor='rgba(255,255,255,0)',
            borderwidth=0,
            itemwidth=30
        ),
        font=dict(size=50, family='GitLab Sans, Arial, sans-serif'),
        plot_bgcolor='#171321',
        paper_bgcolor='#171321',
        width=DEFAULT_PLOT_WIDTH,
        height=DEFAULT_PLOT_HEIGHT,
        margin=dict(l=504, r=800, t=320, b=200)  # Extra right margin for legend
    )
    
    export_plot(fig, show=True, export_path=None)
    
    # Print summary statistics
    print("Tool Usage Comparison Chart created!")
    for run_name, metadata in cross_run_metadata.items():
        clean_name = NAME_MAP.get(run_name, run_name)
        print(f"\n{clean_name}:")
        print(f"  Total tools used: {metadata.total_tools_used}")
        print(f"  Average tools per session: {metadata.avg_tools_used:.1f}")
        top_tools = sorted(metadata.original_proportions.items(), key=lambda x: x[1], reverse=True)[:3]
        print(f"  Top 3 tools: {', '.join([f'{TOOL_NAME_MAP.get(tool, tool)}({pct:.1f}%)' for tool, pct in top_tools])}")

def fixture_passes_chart(cross_run_metadata: dict[str, CrossRunMetadata]):
    """Create a stacked bar chart showing fixture passes per run"""
    
    # Collect all unique fixtures across all runs
    all_fixtures = set()
    for metadata in cross_run_metadata.values():
        all_fixtures.update(metadata.resolved_instances_counts.keys())
    
    # Sort fixtures alphabetically for consistent ordering
    sorted_fixtures = sorted(all_fixtures)
    
    # Color palette for different runs
    colors = {
        'baseline': '#FCA326',      # Orange 01p
        'gkg_only': '#FC6D26',      # Orange 02p
        'baseline_with_gkg': '#E24329'  # Orange 03p
    }
    
    fig = go.Figure()
    
    # Create stacked bars for each fixture
    for run_name, metadata in cross_run_metadata.items():
        clean_run_name = NAME_MAP.get(run_name, run_name.replace('_', ' ').title())
        
        # Get pass counts for each fixture (0 if fixture not resolved in this run)
        pass_counts = [metadata.resolved_instances_counts.get(fixture, 0) for fixture in sorted_fixtures]
        
        fig.add_trace(go.Bar(
            name=clean_run_name,
            x=sorted_fixtures,
            y=pass_counts,
            marker=dict(
                color=colors.get(run_name, '#9C9C9C'),  # Default gray for unknown runs
                line=dict(color='#171321', width=1)
            ),
            text=[str(count) if count > 0 else '' for count in pass_counts],  # Show count if > 0
            textposition='inside',
            textfont=dict(size=32, family='GitLab Sans, Arial, sans-serif', color='white', weight='bold'),
            hovertemplate='<b>%{fullData.name}</b><br>%{x}<br>Passes: %{y}<extra></extra>'
        ))
    
    # Clean up fixture names for x-axis labels
    clean_fixture_names = []
    for fixture in sorted_fixtures:
        # Extract just the repository and issue number for cleaner display
        # e.g., "pydicom__pydicom-1694" -> "pydicom-1694"
        if '__' in fixture:
            parts = fixture.split('__')
            if len(parts) >= 2:
                repo_part = parts[1]  # Take the part after '__'
                clean_fixture_names.append(repo_part)
            else:
                clean_fixture_names.append(fixture)
        else:
            clean_fixture_names.append(fixture)
    
    # Update layout
    fig.update_layout(
        title={
            'text': 'Fixture Passes by Run<br><span style="font-size: 67px; font-weight: 400;">Number of Successful Resolutions per SWE-Bench Instance</span>',
            'font': {'size': 101, 'family': 'GitLab Sans, Arial, sans-serif', 'color': 'white', 'weight': 600},
            'x': 0.5,
            'y': 0.95
        },
        barmode='stack',
        barcornerradius=8,
        xaxis=dict(
            title='SWE-Bench Fixtures',
            title_font=dict(size=76, family='GitLab Sans, Arial, sans-serif', color='white', weight=500),
            title_standoff=75,
            tickfont=dict(size=42, family='GitLab Sans, Arial, sans-serif', color='white'),
            tickangle=45,  # Angle the labels for better readability
            showgrid=False,
            ticktext=clean_fixture_names,
            tickvals=sorted_fixtures
        ),
        yaxis=dict(
            title='Number of Passes',
            title_font=dict(size=76, family='GitLab Sans, Arial, sans-serif', color='white', weight=500),
            title_standoff=75,
            tickfont=dict(size=56, family='GitLab Sans, Arial, sans-serif', color='white'),
            showgrid=True,
            gridcolor='rgba(255,255,255,0.1)',
            dtick=1  # Show integer ticks only
        ),
        legend=dict(
            orientation="v",
            yanchor="top",
            y=0.98,
            xanchor="left", 
            x=1.02,
            font=dict(size=56, family='GitLab Sans, Arial, sans-serif', color='white'),
            bgcolor='rgba(255,255,255,0)',
            borderwidth=0,
            itemwidth=30
        ),
        font=dict(size=50, family='GitLab Sans, Arial, sans-serif'),
        plot_bgcolor='#171321',
        paper_bgcolor='#171321',
        width=DEFAULT_PLOT_WIDTH,
        height=DEFAULT_PLOT_HEIGHT,
        margin=dict(l=504, r=800, t=420, b=300)  # Extra bottom margin for angled labels
    )
    
    export_plot(fig, show=True, export_path=None)
    
    # Print summary statistics
    print("Fixture Passes Chart created!")
    for run_name, metadata in cross_run_metadata.items():
        clean_name = NAME_MAP.get(run_name, run_name)
        total_passes = sum(metadata.resolved_instances_counts.values())
        unique_fixtures = len([count for count in metadata.resolved_instances_counts.values() if count > 0])
        print(f"\n{clean_name}:")
        print(f"  Total fixture passes: {total_passes}")
        print(f"  Unique fixtures resolved: {unique_fixtures}")
        print(f"  Pass rate: {metadata.pass_rate}%")

def file_access_timeline_chart(cross_run_metadata: dict[str, CrossRunMetadata]):
    """Create line charts showing file access patterns over time with patch file highlights"""
    
    # Create subplots for each pipeline type
    fig = make_subplots(
        rows=3, cols=1,
        subplot_titles=[NAME_MAP.get(name, name.replace('_', ' ').title()) for name in cross_run_metadata.keys()],
        vertical_spacing=0.08,
        shared_xaxes=True
    )
    
    colors = {
        'baseline': '#FCA326',
        'gkg_only': '#FC6D26', 
        'baseline_with_gkg': '#E24329'
    }
    
    row_idx = 1
    for run_name, metadata in cross_run_metadata.items():
        clean_run_name = NAME_MAP.get(run_name, run_name.replace('_', ' ').title())
        
        print(f"DEBUG: {run_name} has {len(metadata.session_data)} sessions")
        
        # Process each session (which represents a fixture/problem)
        for session_idx, session in enumerate(metadata.session_data):
            # Extract fixture name from session data
            fixture_name = session.fixture.instance_id if hasattr(session, 'fixture') and hasattr(session.fixture, 'instance_id') else f'Session {session_idx + 1}'
            
            print(f"DEBUG: Processing {fixture_name} with {len(session.file_access_order)} file accesses")
            
            # Skip sessions with no file accesses
            if len(session.file_access_order) == 0:
                continue
                
            # Create timeline showing efficiency: normalize by percentage of total session
            max_files = len(session.file_access_order)
            tool_call_numbers = [(i+1)/max_files * 100 for i in range(max_files)]  # Percentage through session
            file_counts = list(range(1, max_files + 1))
            
            # Find patch file access points
            patch_access_points = []
            for i, file_path in enumerate(session.file_access_order):
                if file_path in session.patch_paths:
                    x_coord = (i+1)/max_files * 100  # Percentage through session
                    y_coord = i + 1  # Cumulative files accessed
                    patch_access_points.append((x_coord, y_coord))
            
            # Vary the color slightly for each session to see multiple lines
            base_color = colors.get(run_name, '#9C9C9C')
            # Create slight variations in the color
            import colorsys
            rgb = tuple(int(base_color[i:i+2], 16) for i in (1, 3, 5))
            hsv = colorsys.rgb_to_hsv(rgb[0]/255, rgb[1]/255, rgb[2]/255)
            # Vary hue slightly based on session index
            new_hsv = (hsv[0] + (session_idx * 0.02) % 1, hsv[1], hsv[2])
            new_rgb = colorsys.hsv_to_rgb(*new_hsv)
            line_color = f"rgb({int(new_rgb[0]*255)},{int(new_rgb[1]*255)},{int(new_rgb[2]*255)})"
            
            # Add main line for this fixture
            fig.add_trace(
                go.Scatter(
                    x=tool_call_numbers,
                    y=file_counts,
                    mode='lines+markers',
                    name=f'{clean_run_name} - {fixture_name}',
                    line=dict(color=line_color, width=2),
                    marker=dict(size=3, color=line_color),
                    opacity=0.7,
                    showlegend=False,  # Don't show individual fixture lines in legend
                    legendgroup=run_name,
                    hovertemplate=f'<b>Fixture:</b> {fixture_name}<br><b>Progress:</b> %{{x:.1f}}%<br><b>Files Accessed:</b> %{{y}}<extra></extra>'
                ),
                row=row_idx, col=1
            )
            
            # Add patch file highlights
            if patch_access_points:
                patch_x = [point[0] for point in patch_access_points]
                patch_y = [point[1] for point in patch_access_points] 
                
                fig.add_trace(
                    go.Scatter(
                        x=patch_x,
                        y=patch_y,
                        mode='markers',
                        name='Patch File Access' if session_idx == 0 and row_idx == 1 else '',
                        marker=dict(
                            size=8,
                            color='#DD2B0E',  # GitLab Red
                            symbol='star',
                            line=dict(width=1, color='white')
                        ),
                        showlegend=session_idx == 0 and row_idx == 1,
                        hovertemplate=f'<b>Patch File Accessed!</b><br><b>Fixture:</b> {fixture_name}<br><b>Progress:</b> %{{x:.1f}}%<br><b>Files Accessed:</b> %{{y}}<extra></extra>'
                    ),
                    row=row_idx, col=1
                )
        
        # Add a dummy trace for legend
        fig.add_trace(
            go.Scatter(
                x=[None],
                y=[None],
                mode='lines',
                name=clean_run_name,
                line=dict(color=colors.get(run_name, '#9C9C9C'), width=4),
                showlegend=True,
                legendgroup=run_name
            ),
            row=row_idx, col=1
        )
        
        row_idx += 1
    
    # Update layout
    fig.update_layout(
        title={
            'text': 'File Access Efficiency by Pipeline Type<br><span style="font-size: 67px; font-weight: 400;">File Access Patterns vs Session Progress with Patch File Highlights</span>',
            'font': {'size': 101, 'family': 'GitLab Sans, Arial, sans-serif', 'color': 'white', 'weight': 600},
            'x': 0.5,
            'y': 0.95
        },
        legend=dict(
            orientation="v",
            yanchor="top",
            y=0.98,
            xanchor="left",
            x=1.02,
            font=dict(size=48, family='GitLab Sans, Arial, sans-serif', color='white'),
            bgcolor='rgba(255,255,255,0)',
            borderwidth=0,
            itemwidth=30
        ),
        font=dict(size=50, family='GitLab Sans, Arial, sans-serif'),
        plot_bgcolor='#171321',
        paper_bgcolor='#171321',
        width=DEFAULT_PLOT_WIDTH,
        height=DEFAULT_PLOT_HEIGHT,
        margin=dict(l=504, r=800, t=420, b=200)
    )
    
    # Update x and y axis labels for all subplots
    for i in range(1, 4):
        fig.update_xaxes(
            title_text='Progress Through Session (%)' if i == 3 else '',
            title_font=dict(size=76, family='GitLab Sans, Arial, sans-serif', color='white', weight=500),
            title_standoff=75,
            tickfont=dict(size=56, family='GitLab Sans, Arial, sans-serif', color='white'),
            showgrid=True,
            gridcolor='rgba(255,255,255,0.1)',
            range=[0, 100],
            row=i, col=1
        )
        
        fig.update_yaxes(
            title_text='',  # Remove the messy title
            tickfont=dict(size=48, family='GitLab Sans, Arial, sans-serif', color='white'),
            showgrid=True,
            gridcolor='rgba(255,255,255,0.1)',
            type='log',
            dtick=1,  # Show every power of 10 (1, 10, 100, etc.)
            row=i, col=1
        )
    
    # Update subplot titles
    for i, annotation in enumerate(fig['layout']['annotations']):
        annotation['font'] = dict(size=84, family='GitLab Sans, Arial, sans-serif', color='white', weight=600)
    
    export_plot(fig, show=True, export_path=None)
    
    # Print summary statistics
    print("File Access Timeline Chart created!")
    for run_name, metadata in cross_run_metadata.items():
        clean_name = NAME_MAP.get(run_name, run_name)
        avg_files_accessed = sum(len(session.file_access_order) for session in metadata.session_data) / len(metadata.session_data)
        avg_patch_accesses = sum(len([f for f in session.file_access_order if f in session.patch_paths]) for session in metadata.session_data) / len(metadata.session_data)
        print(f"\n{clean_name}:")
        print(f"  Average files accessed per session: {avg_files_accessed:.1f}")
        print(f"  Average patch file accesses per session: {avg_patch_accesses:.1f}")
        print(f"  Sessions with patch access: {sum(1 for session in metadata.session_data if any(f in session.patch_paths for f in session.file_access_order))}/{len(metadata.session_data)}")