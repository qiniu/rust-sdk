use mime::Mime;
use qiniu_apis::{
    http::Extensions,
    http_client::{FileName, RegionsProvider},
};
use qiniu_utils::ObjectName;
use std::{collections::HashMap, mem::take, time::Duration};

#[derive(Debug, Default)]
pub struct ObjectParams {
    region_provider: Option<Box<dyn RegionsProvider>>,
    object_name: Option<ObjectName>,
    file_name: Option<FileName>,
    content_type: Option<Mime>,
    metadata: HashMap<String, String>,
    custom_vars: HashMap<String, String>,
    uploaded_part_ttl: Duration,
    extensions: Extensions,
}

impl ObjectParams {
    #[inline]
    pub fn builder() -> ObjectParamsBuilder {
        Default::default()
    }

    #[inline]
    pub fn region_provider(&self) -> Option<&dyn RegionsProvider> {
        self.region_provider.as_deref()
    }

    #[inline]
    pub fn region_provider_mut(&mut self) -> &mut Option<Box<dyn RegionsProvider>> {
        &mut self.region_provider
    }

    #[inline]
    pub fn object_name(&self) -> Option<&str> {
        self.object_name.as_deref()
    }

    #[inline]
    pub fn object_name_mut(&mut self) -> &mut Option<ObjectName> {
        &mut self.object_name
    }

    #[inline]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    #[inline]
    pub fn file_name_mut(&mut self) -> &mut Option<FileName> {
        &mut self.file_name
    }

    #[inline]
    pub fn content_type(&self) -> Option<&Mime> {
        self.content_type.as_ref()
    }

    #[inline]
    pub fn content_type_mut(&mut self) -> &mut Option<Mime> {
        &mut self.content_type
    }

    #[inline]
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    #[inline]
    pub fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }

    #[inline]
    pub fn custom_vars(&self) -> &HashMap<String, String> {
        &self.custom_vars
    }

    #[inline]
    pub fn custom_vars_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.custom_vars
    }

    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    #[inline]
    pub fn uploaded_part_ttl(&self) -> Duration {
        self.uploaded_part_ttl
    }

    #[inline]
    pub fn uploaded_part_ttl_mut(&mut self) -> &mut Duration {
        &mut self.uploaded_part_ttl
    }
}

#[derive(Debug)]
pub struct ObjectParamsBuilder(ObjectParams);

impl Default for ObjectParamsBuilder {
    fn default() -> Self {
        Self(ObjectParams {
            uploaded_part_ttl: Duration::from_secs(5 * 86400),
            region_provider: Default::default(),
            object_name: Default::default(),
            file_name: Default::default(),
            content_type: Default::default(),
            metadata: Default::default(),
            custom_vars: Default::default(),
            extensions: Default::default(),
        })
    }
}

impl ObjectParamsBuilder {
    #[inline]
    pub fn region_provider(&mut self, region_provider: impl RegionsProvider + 'static) -> &mut Self {
        self.0.region_provider = Some(Box::new(region_provider));
        self
    }

    #[inline]
    pub fn object_name(&mut self, object_name: impl Into<ObjectName>) -> &mut Self {
        self.0.object_name = Some(object_name.into());
        self
    }

    #[inline]
    pub fn file_name(&mut self, file_name: impl Into<FileName>) -> &mut Self {
        self.0.file_name = Some(file_name.into());
        self
    }

    #[inline]
    pub fn content_type(&mut self, content_type: Mime) -> &mut Self {
        self.0.content_type = Some(content_type);
        self
    }

    #[inline]
    pub fn metadata(&mut self, metadata: HashMap<String, String>) -> &mut Self {
        self.0.metadata = metadata;
        self
    }

    #[inline]
    pub fn insert_metadata<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) -> &mut Self {
        self.0.metadata.insert(key.into(), value.into());
        self
    }

    #[inline]
    pub fn custom_vars(&mut self, custom_vars: HashMap<String, String>) -> &mut Self {
        self.0.custom_vars = custom_vars;
        self
    }

    #[inline]
    pub fn insert_custom_var<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) -> &mut Self {
        self.0.custom_vars.insert(key.into(), value.into());
        self
    }

    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.0.extensions = extensions;
        self
    }

    #[inline]
    pub fn insert_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.0.extensions.insert(val);
        self
    }

    #[inline]
    pub fn uploaded_part_ttl(&mut self, uploaded_part_ttl: Duration) -> &mut Self {
        self.0.uploaded_part_ttl = uploaded_part_ttl;
        self
    }

    #[inline]
    pub fn build(&mut self) -> ObjectParams {
        take(&mut self.0)
    }
}
