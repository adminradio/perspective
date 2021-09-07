////////////////////////////////////////////////////////////////////////////////
//
// Copyright (c) 2018, the Perspective Authors.
//
// This file is part of the Perspective library, distributed under the terms
// of the Apache License 2.0.  The full license can be found in the LICENSE
// file.

use crate::config::*;
use crate::custom_elements::filter_dropdown::*;
use crate::dragdrop::*;
use crate::renderer::*;
use crate::session::*;
use crate::*;

use super::containers::dragdrop_list::*;
use super::containers::dropdown::*;

use chrono::{Local, NaiveDate, TimeZone};
use web_sys::*;
use yew::prelude::*;

/// A control for a single filter condition.
pub struct FilterItem {
    props: FilterItemProperties,
    link: ComponentLink<FilterItem>,
    input: String,
}

pub enum FilterItemMsg {
    FilterInput((usize, String), String, HtmlElement),
    Close,
    FilterOpSelect(FilterOp),
    FilterKeyDown(u32),
}

#[derive(Properties, Clone)]
pub struct FilterItemProperties {
    pub filter: Filter,
    pub idx: usize,
    pub filter_dropdown: FilterDropDownElement,
    pub on_keydown: Callback<String>,
    pub session: Session,
    pub renderer: Renderer,
    pub dragdrop: DragDrop,
}

derive_renderable_props!(FilterItemProperties);

impl DragDropListItemProps for FilterItemProperties {
    type Item = Filter;

    fn get_item(&self) -> Filter {
        self.filter.clone()
    }
}

impl FilterItemProperties {
    /// Does this filter item get a "suggestions" auto-complete modal?
    fn is_suggestable(&self) -> bool {
        self.filter.1 == FilterOp::EQ && self.get_filter_type() == Type::String
    }

    /// Get this filter's type, e.g. the type of the column.
    fn get_filter_type(&self) -> Type {
        self.session
            .metadata()
            .get_column_table_type(&self.filter.0)
            .unwrap()
    }

    /// Get the allowed `FilterOp`s for this filter.
    fn get_filter_ops(&self) -> Vec<FilterOp> {
        match self.get_filter_type() {
            Type::String => vec![
                FilterOp::EQ,
                FilterOp::NE,
                FilterOp::GT,
                FilterOp::GTE,
                FilterOp::LT,
                FilterOp::LTE,
                FilterOp::BeginsWith,
                FilterOp::Contains,
                FilterOp::EndsWith,
                FilterOp::In,
                FilterOp::IsNotNull,
                FilterOp::IsNull,
            ],
            _ => vec![
                FilterOp::EQ,
                FilterOp::NE,
                FilterOp::GT,
                FilterOp::GTE,
                FilterOp::LT,
                FilterOp::LTE,
                FilterOp::IsNotNull,
                FilterOp::IsNull,
            ],
        }
    }

    /// Update the filter comparison operator.
    ///
    /// # Arguments
    /// - `op` The new `FilterOp`.
    fn update_filter_op(&self, op: FilterOp) {
        let ViewConfig { mut filter, .. } = self.session.get_view_config();
        let filter_item = &mut filter.get_mut(self.idx).expect("Filter on no column");
        filter_item.1 = op;
        let update = ViewConfigUpdate {
            filter: Some(filter),
            ..ViewConfigUpdate::default()
        };

        self.update_and_render(update);
    }

    /// Update the filter Value.
    ///
    /// # Arguments
    /// - `val` The new filter value.
    fn update_filter_value(&self, val: String) {
        let ViewConfig { mut filter, .. } = self.session.get_view_config();
        let filter_item = &mut filter.get_mut(self.idx).expect("Filter on no column");
        match filter_item.1 {
            FilterOp::In => {
                filter_item.2 = FilterTerm::Array(
                    val.split(',')
                        .map(|x| Scalar::String(x.trim().to_owned()))
                        .collect(),
                );
            }
            _ => match self.get_filter_type() {
                Type::String => {
                    filter_item.2 = FilterTerm::Scalar(Scalar::String(val));
                }
                Type::Integer => {
                    if val.is_empty() {
                        filter_item.2 = FilterTerm::Scalar(Scalar::Null);
                    } else if let Ok(num) = val.parse::<f64>() {
                        filter_item.2 = FilterTerm::Scalar(Scalar::Float(num.floor()));
                    }
                }
                Type::Float => {
                    if val.is_empty() {
                        filter_item.2 = FilterTerm::Scalar(Scalar::Null);
                    } else if let Ok(num) = val.parse::<f64>() {
                        filter_item.2 = FilterTerm::Scalar(Scalar::Float(num));
                    }
                }
                Type::Date => {
                    filter_item.2 = FilterTerm::Scalar(match NaiveDate::parse_from_str(
                        &val, "%Y-%m-%d",
                    ) {
                        Ok(ref posix) => match posix.and_hms_opt(0, 0, 0) {
                            Some(x) => Scalar::DateTime(x.timestamp_millis() as u64),
                            None => Scalar::Null,
                        },
                        _ => Scalar::Null,
                    })
                }
                _ => {}
            },
        }

        let update = ViewConfigUpdate {
            filter: Some(filter),
            ..ViewConfigUpdate::default()
        };

        self.update_and_render(update);
    }
}

type FilterOpSelector = DropDown<FilterOp>;

impl Component for FilterItem {
    type Message = FilterItemMsg;
    type Properties = FilterItemProperties;

    fn create(props: FilterItemProperties, link: ComponentLink<Self>) -> Self {
        let input = match &props.filter.2 {
            FilterTerm::Scalar(Scalar::DateTime(x)) => {
                let time = Local
                    .timestamp(*x as i64 / 1000, (*x as u32 % 1000) * 1000)
                    .format("%Y-%m-%d")
                    .to_string();
                time
            }
            x => format!("{}", x),
        };

        FilterItem { props, link, input }
    }

    fn update(&mut self, msg: FilterItemMsg) -> bool {
        match msg {
            FilterItemMsg::FilterInput(column, input, target) => {
                self.input = input.clone();
                if self.props.is_suggestable() {
                    self.props.filter_dropdown.autocomplete(
                        column,
                        input.clone(),
                        target,
                    );
                }

                self.props.update_filter_value(input);
                false
            }
            FilterItemMsg::FilterKeyDown(40) => {
                if self.props.is_suggestable() {
                    self.props.filter_dropdown.item_down();
                    self.props.filter_dropdown.item_select();
                }
                false
            }
            FilterItemMsg::FilterKeyDown(38) => {
                if self.props.is_suggestable() {
                    self.props.filter_dropdown.item_up();
                    self.props.filter_dropdown.item_select();
                }
                false
            }
            FilterItemMsg::Close => {
                self.props.filter_dropdown.hide().unwrap();
                false
            }
            FilterItemMsg::FilterKeyDown(13) => {
                if self.props.is_suggestable() {
                    self.props.filter_dropdown.item_select();
                    self.props.filter_dropdown.hide().unwrap();
                }
                false
            }
            FilterItemMsg::FilterKeyDown(_) => {
                if self.props.is_suggestable() {
                    self.props.filter_dropdown.reautocomplete();
                }
                false
            }
            FilterItemMsg::FilterOpSelect(op) => {
                self.props.update_filter_op(op);
                false
            }
        }
    }

    fn change(&mut self, props: FilterItemProperties) -> bool {
        match &props.filter.2 {
            FilterTerm::Scalar(Scalar::DateTime(x)) => {
                let rescaled = *x as i64;
                if rescaled > 0 {
                    if let chrono::LocalResult::Single(x) = Local.timestamp_opt(
                        rescaled / 1000,
                        ((rescaled % 1000) * 1000) as u32,
                    ) {
                        self.input = x.format("%Y-%m-%d").to_string();
                    }
                }
            }
            x => self.input = format!("{}", x),
        };

        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let idx = self.props.idx;
        let filter = self.props.filter.clone();
        let column = filter.0.to_owned();
        let col_type = self
            .props
            .session
            .metadata()
            .get_column_table_type(&column)
            .unwrap();

        let select = self.link.callback(FilterItemMsg::FilterOpSelect);

        let noderef = NodeRef::default();
        let input = self.link.callback({
            let noderef = noderef.clone();
            let column = column.clone();
            move |input: InputData| {
                let target = noderef.cast::<HtmlElement>().unwrap();
                FilterItemMsg::FilterInput((idx, column.clone()), input.value, target)
            }
        });

        let focus = self.link.callback({
            let noderef = noderef.clone();
            let input = self.input.clone();
            move |_: FocusEvent| {
                let target = noderef.cast::<HtmlElement>().unwrap();
                FilterItemMsg::FilterInput((idx, column.clone()), input.clone(), target)
            }
        });

        let blur = self.link.callback(|_| FilterItemMsg::Close);
        let keydown = self.link.callback(move |event: KeyboardEvent| {
            FilterItemMsg::FilterKeyDown(event.key_code())
        });

        let dragref = NodeRef::default();
        let dragstart = Callback::from({
            let event_name = self.props.filter.0.to_owned();
            let dragref = dragref.clone();
            let dragdrop = self.props.dragdrop.clone();
            move |event: DragEvent| {
                let elem = dragref.cast::<HtmlElement>().unwrap();
                event.data_transfer().unwrap().set_drag_image(&elem, 0, 0);
                dragdrop.drag_start(
                    event_name.to_string(),
                    DragEffect::Move(DropAction::Filter),
                )
            }
        });

        let type_class = match col_type {
            Type::Float | Type::Integer => "num-filter",
            Type::String => "string-filter",
            _ => "",
        };

        let input_elem = match col_type {
            Type::Integer => html! {
                <input
                    type="number"
                    placeholder="Value"
                    class="num-filter"
                    step="1"
                    ref={ noderef.clone() }
                    onkeydown={ keydown }
                    onfocus={ focus }
                    onblur={ blur }
                    value={ self.input.clone() }
                    oninput={ input }/>
            },
            Type::Float => html! {
                <input
                    type="number"
                    placeholder="Value"
                    class="num-filter"
                    ref={ noderef.clone() }
                    onkeydown={ keydown }
                    onfocus={ focus }
                    onblur={ blur }
                    value={ self.input.clone() }
                    oninput={ input }/>
            },
            Type::String => html! {
                <input
                    type="text"
                    size="4"
                    placeholder="Value"
                    class="string-filter"
                    // TODO This is dirty and it may not work in the future.
                    onInput="this.parentNode.dataset.value=this.value"
                    ref={ noderef.clone() }
                    onkeydown={ keydown }
                    onfocus={ focus }
                    onblur={ blur }
                    value={ self.input.clone() }
                    oninput={ input }/>
            },
            Type::Date => html! {
                <input
                    type="date"
                    placeholder="Value"
                    class="date-filter"
                    ref={ noderef.clone() }
                    onkeydown={ keydown }
                    onfocus={ focus }
                    onblur={ blur }
                    value={ self.input.clone() }
                    oninput={ input }/>
            },
            Type::Datetime => html! {
                <>
                    <input
                        type="date"
                        placeholder="Value"
                        class="date-filter"
                        ref={ noderef.clone() }
                        onkeydown={ keydown.clone() }
                        onfocus={ focus.clone() }
                        onblur={ blur.clone() }
                        // value={ self.input.clone() }
                        oninput={ input.clone() }/>

                    <input
                        type="time"
                        placeholder="Value"
                        class="time-filter"
                        // ref={ noderef.clone() }
                        onkeydown={ keydown }
                        onfocus={ focus }
                        onblur={ blur }
                        // value={ self.input.clone() }
                        oninput={ input }/>
                </>
            },
            _ => {
                html! {}
            }
        };

        html! {
            <>
                <span
                    draggable="true"
                    ref={ dragref }
                    ondragstart={ dragstart }>
                    {
                        filter.0.to_owned()
                    }
                </span>
                <FilterOpSelector
                    class="filter-op-selector"
                    auto_resize=true
                    values={ self.props.get_filter_ops() }
                    selected={ filter.1 }
                    on_select={ select }>
                </FilterOpSelector>
                <label
                    class={ format!("input-sizer {}", type_class) }
                    data-value={ format!("{}", filter.2) }>
                    {
                        input_elem
                    }
                </label>
            </>
        }
    }
}