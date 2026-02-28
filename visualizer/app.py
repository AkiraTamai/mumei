import streamlit as st
import json
import pandas as pd

st.set_page_config(page_title="Mumei Visualizer", page_icon="🗡️")

st.title("🗡️ Mumei Visualizer")
st.subheader("Formal Verification Inspection Dashboard")

try:
    with open("report.json", "r") as f:
        data = json.load(f)
except FileNotFoundError:
    st.info("No verification reports found. Run the Mumei compiler first.")
    st.stop()

# 状態の表示
if data["status"] == "failed":
    st.error(f"❌ Verification Failed: Atom '{data['atom']}' is flawed.")

    col1, col2 = st.columns(2)
    with col1:
        st.metric("Counter-example: a", data["input_a"])
    with col2:
        st.metric("Counter-example: b", data["input_b"])

    st.warning(f"**Reason:** {data['reason']}")

    # AIへの修正指示用プロンプトの自動生成
    st.code(f"""
    # AI Fix Suggestion:
    The atom '{data['atom']}' failed verification when b={data['input_b']}.
    Please update the 'requires' clause to handle this case.
    """, language="markdown")
else:
    st.success(f"✅ Atom '{data['atom']}' is mathematically pure.")