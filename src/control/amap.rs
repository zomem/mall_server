use actix_web::{Error, error};
use serde::{Deserialize, Serialize};

use crate::{
    common::{AMAP_WEB_SERVER_KEY, AMAP_WEB_URL},
    utils::utils::{keep_decimal, log_err},
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AmapGeocodeRegeo {
    pub status: String,
    pub regeocode: AmapRegeocode,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AmapRegeocode {
    pub address_component: AmapAddressCom,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AmapAddressCom {
    pub township: String,
    pub street_number: AmapStreetNumber,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AmapStreetNumber {
    pub street: String,
    pub number: String,
}
/// 高德地图，坐标获取地址信息 (lat, lng)
pub async fn amap_geocode_regeo(loc: (f64, f64)) -> anyhow::Result<AmapGeocodeRegeo, Error> {
    let url = AMAP_WEB_URL.to_string()
        + "/geocode/regeo?"
        + "key="
        + AMAP_WEB_SERVER_KEY
        + "&location="
        + &loc.1.to_string()
        + ","
        + &loc.0.to_string();
    let res: AmapGeocodeRegeo = reqwest::get(&url)
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;
    if res.status != "1".to_string() {
        // 请求失败
        return Err(error::ErrorBadGateway(log_err(&res, "高德地图请求错误")));
    }
    Ok(res)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AmapDrive {
    pub route: AmapRoute,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AmapRoute {
    /// 打车费用 元
    pub taxi_cost: f64,
    pub paths: Vec<AmapRoutePath>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AmapRoutePath {
    /// 行驶距离 千米 km
    pub distance: f64,
    /// 预计行驶时间 秒 s
    pub duration: f64,
}
/// 高德地图，路线规划，驾车路线距离计算
/// lat,lng
#[allow(unused)]
pub async fn amap_drive_distance(
    start: (f64, f64),
    end: (f64, f64),
) -> anyhow::Result<AmapDrive, Error> {
    #[derive(Serialize, Deserialize, Debug)]
    struct AmapGet {
        status: String,
        route: AmapRouteGet,
    }
    #[derive(Serialize, Deserialize, Debug)]
    struct AmapRouteGet {
        /// 打车费用 元
        taxi_cost: String,
        paths: Vec<AmapRoutePathGet>,
    }
    #[derive(Serialize, Deserialize, Debug)]
    struct AmapRoutePathGet {
        /// 行驶距离 米
        distance: String,
        /// 预计行驶时间 秒
        duration: String,
    }
    let url = AMAP_WEB_URL.to_string()
        + "/direction/driving?"
        + "key="
        + AMAP_WEB_SERVER_KEY
        + "&origin="
        + &start.1.to_string()
        + ","
        + &start.0.to_string()
        + "&destination="
        + &end.1.to_string()
        + ","
        + &end.0.to_string()
        + "&nosteps=1&extensions=all";
    let res: AmapGet = reqwest::get(&url)
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;
    if res.status != "1".to_string() {
        // 请求失败
        return Err(error::ErrorBadGateway(log_err(&res, "高德地图请求错误")));
    }
    Ok(AmapDrive {
        route: AmapRoute {
            taxi_cost: keep_decimal(res.route.taxi_cost.parse::<f64>().unwrap()),
            paths: if res.route.paths.len() > 0 {
                vec![AmapRoutePath {
                    distance: keep_decimal(
                        res.route.paths[0].distance.parse::<f64>().unwrap() / 1000.,
                    ),
                    duration: res.route.paths[0].duration.parse::<f64>().unwrap(),
                }]
            } else {
                vec![]
            },
        },
    })
}

#[cfg(test)]
mod test {
    use super::{amap_drive_distance, amap_geocode_regeo};

    #[tokio::test]
    async fn test_amap() {
        let _am = amap_drive_distance((39.989643, 116.481028), (40.004717, 116.465302))
            .await
            .unwrap();
        let am = amap_geocode_regeo((39.989643, 116.481028)).await.unwrap();
        println!("am,,,,  {:#?}", am);
    }
}
