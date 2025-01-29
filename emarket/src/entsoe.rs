use async_trait::async_trait;
use chrono::NaiveDateTime;
use emarket::data::{Data, Loader};

use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;

use serde_xml_rs::from_str;

#[derive(Debug)]
pub struct EntSOE {
    url: String,
    domain: String,
    key: String,
    document: String,
    client: ClientWithMiddleware,
}

impl EntSOE {
    pub fn new(document: &str, domain: &str, key: &str) -> Result<EntSOE, Box<dyn Error>> {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
        let client = Client::builder()
            .pool_max_idle_per_host(5)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        let client_with_retry = ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        // let client = reqwest::Client::builder()
        //     .connect_timeout(Duration::from_secs(5))
        //     .timeout(Duration::from_secs(15))
        //     .build()?;
        // let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
        // let client = ClientBuilder::new(client)
        //     .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        //     .build();

        Ok(EntSOE {
            url: "https://web-api.tp.entsoe.eu/api".to_string(),
            client: client_with_retry,
            document: document.to_string(),
            domain: domain.to_string(),
            key: key.to_string(),
        })
    }

    async fn text(
        &self,
        url: &str,
        want: StatusCode,
    ) -> std::result::Result<String, Box<dyn Error>> {
        tracing::debug!(url, "calling...");
        let response = self.client.get(url).send().await?;

        // Validate the status code
        let status = response.status();
        let txt = response.text().await?;
        tracing::trace!(txt, status = status.as_u16(), "got");
        if status != want {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("status code: {}, body: {}", status, txt),
            )));
        }
        Ok(txt)
    }
}

#[async_trait]
impl Loader for EntSOE {
    async fn live(&self) -> std::result::Result<String, Box<dyn Error>> {
        let url = format!("{}?securityToken={}", self.url, self.key);
        let content = self.text(&url, StatusCode::BAD_REQUEST).await?; // 400 is expected, NO TIME DEFINED
        tracing::trace!(content, "got");
        Ok(content)
    }
    async fn retrieve(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> std::result::Result<Vec<Data>, Box<dyn Error>> {
        //https://transparency.entsoe.eu/api?securityToken=$(TOKEN)&documentType=A44&in_Domain=10YLT-1001A0008Q&out_Domain=10YLT-1001A0008Q&periodStart=202112312300&periodEnd=202212312300
        let url = format!(
            "{}?securityToken={}&documentType={}&in_Domain={}&out_Domain={}&periodStart={}&periodEnd={}",
            self.url, self.key, self.document, self.domain, self.domain, to_time_str(from), to_time_str(to));
        let txt = self.text(&url, StatusCode::OK).await?;
        let in_res = from_str::<EntSOEDoc>(txt.as_str())?;
        tracing::debug!(len = in_res.timeseries.len(), "got timeseries");
        let res = map_to_data(in_res)?;
        tracing::debug!(len = res.len(), "extracted points");
        Ok(res)
    }
}

fn to_time_str(t: NaiveDateTime) -> String {
    t.format("%Y%m%d%H%M").to_string()
}

fn map_to_data(doc: EntSOEDoc) -> Result<Vec<Data>, Box<dyn Error>> {
    let res = doc
        .timeseries
        .iter()
        .flat_map(|t| &t.periods)
        .scan((), |_, p| to_data(p).ok())
        .flatten()
        .collect();
    Ok(res)
}

fn to_data(p: &EntSOEPeriod) -> Result<Vec<Data>, Box<dyn Error>> {
    let time = NaiveDateTime::parse_from_str(&p.time_interval.start, "%Y-%m-%dT%H:%MZ")?;
    let res = p
        .points
        .iter()
        .map(|p| Data {
            at: time + chrono::Duration::hours(i64::from(p.position) - 1),
            price: p.price,
        })
        .collect();
    Ok(res)
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct EntSOEDoc {
    #[serde(rename = "type", default)]
    pub doc_type: String,
    #[serde(rename = "TimeSeries", default)]
    pub timeseries: Vec<EntSOETimeseries>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct EntSOETimeseries {
    #[serde(rename = "mRID", default)]
    pub id: String,
    #[serde(rename = "Period", default)]
    pub periods: Vec<EntSOEPeriod>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct EntSOEPeriod {
    #[serde(rename = "timeInterval")]
    pub time_interval: EntSOETimeInterval,
    #[serde(rename = "Point")]
    pub points: Vec<EntSOEPoint>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct EntSOETimeInterval {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct EntSOEPoint {
    #[serde(rename = "position", default)]
    pub position: u32,
    #[serde(rename = "price.amount", default)]
    pub price: f64,
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use chrono::DateTime;

    use crate::entsoe::{map_to_data, to_time_str, EntSOEDoc};
    use serde_xml_rs::from_str;

    fn one_sample() -> &'static str {
        r#" <Publication_MarketDocument xmlns="urn:iec62325.351:tc57wg16:451-3:publicationdocument:7:0">
    	<mRID>07e8e74a9f4141488a9218781702643d</mRID>
    	<revisionNumber>1</revisionNumber>
    	<type>A44</type>
    	<sender_MarketParticipant.mRID codingScheme="A01">10X1001A1001A450</sender_MarketParticipant.mRID>
    	<sender_MarketParticipant.marketRole.type>A32</sender_MarketParticipant.marketRole.type>
    	<receiver_MarketParticipant.mRID codingScheme="A01">10X1001A1001A450</receiver_MarketParticipant.mRID>
    	<receiver_MarketParticipant.marketRole.type>A33</receiver_MarketParticipant.marketRole.type>
    	<createdDateTime>2023-01-23T06:17:46Z</createdDateTime>
    	<period.timeInterval>
    		<start>2021-12-31T23:00Z</start>
    		<end>2022-01-01T23:00Z</end>
    	</period.timeInterval>
    	<TimeSeries>
    		<mRID>1</mRID>
    		<businessType>A62</businessType>
    		<in_Domain.mRID codingScheme="A01">10YLT-1001A0008Q</in_Domain.mRID>
    		<out_Domain.mRID codingScheme="A01">10YLT-1001A0008Q</out_Domain.mRID>
    		<currency_Unit.name>EUR</currency_Unit.name>
    		<price_Measure_Unit.name>MWH</price_Measure_Unit.name>
    		<curveType>A01</curveType>
    			<Period>
    				<timeInterval>
    					<start>2021-12-31T23:00Z</start>
    					<end>2022-01-01T23:00Z</end>
    				</timeInterval>
    				<resolution>PT60M</resolution>
    					<Point>
    						<position>1</position>
    						<price.amount>50.05</price.amount>
    					</Point>
    					<Point>
    						<position>2</position>
    						<price.amount>41.33</price.amount>
    					</Point>
    			</Period>
    	</TimeSeries>
    </Publication_MarketDocument>"#
    }

    #[test]
    fn deseriarelize_entsoe_doc() {
        let deserialized = from_str::<EntSOEDoc>(one_sample()).unwrap();
        assert_eq!(deserialized.doc_type, "A44");
        assert_eq!(deserialized.timeseries.len(), 1);
        assert_eq!(deserialized.timeseries[0].periods.len(), 1);
        assert_eq!(
            deserialized.timeseries[0].periods[0].time_interval.start,
            "2021-12-31T23:00Z"
        );
        assert_eq!(
            deserialized.timeseries[0].periods[0].time_interval.end,
            "2022-01-01T23:00Z"
        );
        assert_eq!(deserialized.timeseries[0].periods[0].points.len(), 2);
        assert_eq!(deserialized.timeseries[0].periods[0].points[0].position, 1);
        assert_relative_eq!(deserialized.timeseries[0].periods[0].points[0].price, 50.05);
        assert_eq!(deserialized.timeseries[0].periods[0].points[1].position, 2);
        assert_relative_eq!(deserialized.timeseries[0].periods[0].points[1].price, 41.33);
    }

    #[test]
    fn maps_data() {
        let deserialized: EntSOEDoc = from_str(one_sample()).unwrap();
        let res = map_to_data(deserialized).unwrap();
        assert_eq!(res.len(), 2);
        assert_relative_eq!(res[0].price, 50.05);
        assert_eq!(res[0].at.and_utc().timestamp_millis(), 1640991600000);
        assert_relative_eq!(res[1].price, 41.33);
        assert_eq!(res[1].at.and_utc().timestamp_millis(), 1640995200000);
    }

    #[test]
    fn formats_time() {
        assert_eq!(
            to_time_str(
                DateTime::from_timestamp_millis(1640991600000)
                    .unwrap()
                    .naive_utc()
            ),
            "202112312300"
        );
        assert_eq!(
            to_time_str(
                DateTime::from_timestamp_millis(1640995200000)
                    .unwrap()
                    .naive_utc()
            ),
            "202201010000"
        );
    }
}
